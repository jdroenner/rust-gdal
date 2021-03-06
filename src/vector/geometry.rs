use std::ptr::null;
use libc::{c_char, c_int, c_double, c_void};
use std::ffi::CString;
use std::cell::RefCell;
use utils::_string;
use vector::ogr;

/// OGR Geometry
pub struct Geometry {
    c_geometry_ref: RefCell<Option<*const c_void>>,
    owned: bool,
}


impl Geometry {
    pub unsafe fn lazy_feature_geometry() -> Geometry {
        // Geometry objects created with this method map to a Feature's
        // geometry whose memory is managed by the GDAL feature.
        // This object has a tricky lifecycle:
        //
        // * Initially it's created with a null c_geometry
        // * The first time `Feature::geometry` is called, it gets
        //   c_geometry from GDAL and calls `set_c_geometry` with it.
        // * When the Feature is destroyed, this object is also destroyed,
        //   which is good, because that's when c_geometry (which is managed
        //   by the GDAL feature) becomes invalid. Because `self.owned` is
        //   `true`, we don't call `OGR_G_DestroyGeometry`.
        return Geometry{c_geometry_ref: RefCell::new(None), owned: false};
    }

    pub fn has_gdal_ptr(&self) -> bool {
        return self.c_geometry_ref.borrow().is_some();
    }

    pub unsafe fn set_c_geometry(&self, c_geometry: *const c_void) {
        assert!(! self.has_gdal_ptr());
        assert_eq!(self.owned, false);
        *(self.c_geometry_ref.borrow_mut()) = Some(c_geometry);
    }

    unsafe fn with_c_geometry(c_geom: *const c_void, owned: bool) -> Geometry {
        return Geometry{
            c_geometry_ref: RefCell::new(Some(c_geom)),
            owned: owned,
        };
    }

    pub fn empty(wkb_type: c_int) -> Geometry {
        let c_geom = unsafe { ogr::OGR_G_CreateGeometry(wkb_type) };
        assert!(c_geom != null());
        return unsafe { Geometry::with_c_geometry(c_geom, true) };
    }

    /// Create a geometry by parsing a
    /// [WKT](https://en.wikipedia.org/wiki/Well-known_text) string.
    pub fn from_wkt(wkt: &str) -> Geometry {
        let c_wkt = CString::new(wkt.as_bytes()).unwrap();
        let mut c_wkt_ptr: *const c_char = c_wkt.as_ptr();
        let mut c_geom: *const c_void = null();
        let rv = unsafe { ogr::OGR_G_CreateFromWkt(&mut c_wkt_ptr, null(), &mut c_geom) };
        assert_eq!(rv, ogr::OGRERR_NONE);
        return unsafe { Geometry::with_c_geometry(c_geom, true) };
    }

    /// Create a rectangular geometry from West, South, East and North values.
    pub fn bbox(w: f64, s: f64, e: f64, n: f64) -> Geometry {
        Geometry::from_wkt(&format!(
            "POLYGON (({} {}, {} {}, {} {}, {} {}, {} {}))",
            w, n,
            e, n,
            e, s,
            w, s,
            w, n,
        ))
    }

    /// Serialize the geometry as JSON.
    pub fn json(&self) -> String {
        let c_json = unsafe { ogr::OGR_G_ExportToJson(self.c_geometry()) };
        let rv = _string(c_json);
        unsafe { ogr::VSIFree(c_json as *mut c_void) };
        return rv;
    }

    /// Serialize the geometry as WKT.
    pub fn wkt(&self) -> String {
        let mut c_wkt: *const c_char = null();
        let _err = unsafe { ogr::OGR_G_ExportToWkt(self.c_geometry(), &mut c_wkt) };
        assert_eq!(_err, ogr::OGRERR_NONE);
        let wkt = _string(c_wkt);
        unsafe { ogr::OGRFree(c_wkt as *mut c_void) };
        return wkt;
    }

    pub unsafe fn c_geometry(&self) -> *const c_void {
        return self.c_geometry_ref.borrow().unwrap();
    }

    pub unsafe fn into_c_geometry(mut self) -> *const c_void {
        assert!(self.owned);
        self.owned = false;
        return self.c_geometry();
    }

    pub fn set_point_2d(&mut self, i: usize, p: (f64, f64)) {
        let (x, y) = p;
        unsafe { ogr::OGR_G_SetPoint_2D(
            self.c_geometry(),
            i as c_int,
            x as c_double,
            y as c_double,
        ) };
    }

    pub fn get_point(&self, i: i32) -> (f64, f64, f64) {
        let mut x: c_double = 0.;
        let mut y: c_double = 0.;
        let mut z: c_double = 0.;
        unsafe { ogr::OGR_G_GetPoint(self.c_geometry(), i, &mut x, &mut y, &mut z) };
        return (x as f64, y as f64, z as f64);
    }

    pub fn get_point_vec(&self) -> Vec<(f64, f64, f64)> {
        let length = unsafe{ ogr::OGR_G_GetPointCount(self.c_geometry()) };
        return (0..length).map(|i| self.get_point(i)).collect();
    }

    /// Compute the convex hull of this geometry.
    pub fn convex_hull(&self) -> Geometry {
        let c_geom = unsafe { ogr::OGR_G_ConvexHull(self.c_geometry()) };
        return unsafe { Geometry::with_c_geometry(c_geom, true) };
    }

    pub unsafe fn _get_geometry(&self, n: usize) -> Geometry {
        // get the n-th sub-geometry as a non-owned Geometry; don't keep this
        // object for long.
        let c_geom = ogr::OGR_G_GetGeometryRef(self.c_geometry(), n as c_int);
        return Geometry::with_c_geometry(c_geom, false);
    }

    pub fn add_geometry(&mut self, mut sub: Geometry) {
        assert!(sub.owned);
        sub.owned = false;
        let rv = unsafe { ogr::OGR_G_AddGeometryDirectly(
            self.c_geometry(),
            sub.c_geometry(),
        ) };
        assert_eq!(rv, ogr::OGRERR_NONE);
    }
}

impl Drop for Geometry {
    fn drop(&mut self) {
        if self.owned {
            let c_geometry = self.c_geometry_ref.borrow();
            unsafe { ogr::OGR_G_DestroyGeometry(c_geometry.unwrap() as *mut c_void) };
        }
    }
}

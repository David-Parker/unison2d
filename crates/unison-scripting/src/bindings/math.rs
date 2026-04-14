//! Math utility bindings — Color, Rng, and math extensions under `unison.*`.
//!
//! ```lua
//! local c = unison.Color.hex(0xFF0000)
//! local c2 = unison.Color.rgba(0, 1, 0, 1)
//! local blended = c:lerp(c2, 0.5)
//!
//! local rng = unison.Rng.new(42)
//! local x = rng:range(0, 10)
//! local n = rng:range_int(1, 6)
//!
//! local v = unison.math.lerp(0, 100, 0.5)       -- 50
//! local s = unison.math.smoothstep(0, 1, 0.5)   -- 0.5
//! local c = unison.math.clamp(15, 0, 10)        -- 10
//! ```

use mlua::prelude::*;
use unison2d::render::Color;

// ===================================================================
// LuaColor userdata
// ===================================================================

#[derive(Clone, Copy)]
struct LuaColor(Color);

impl LuaUserData for LuaColor {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("r", |_, this| Ok(this.0.r));
        fields.add_field_method_get("g", |_, this| Ok(this.0.g));
        fields.add_field_method_get("b", |_, this| Ok(this.0.b));
        fields.add_field_method_get("a", |_, this| Ok(this.0.a));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("lerp", |_, this, (other, t): (LuaAnyUserData, f32)| {
            let other = other.borrow::<LuaColor>()?;
            let a = this.0;
            let b = other.0;
            Ok(LuaColor(Color::new(
                a.r + (b.r - a.r) * t,
                a.g + (b.g - a.g) * t,
                a.b + (b.b - a.b) * t,
                a.a + (b.a - a.a) * t,
            )))
        });
    }
}

// ===================================================================
// LuaRng userdata (simple xorshift64)
// ===================================================================

struct LuaRng {
    state: u64,
}

impl LuaRng {
    fn next_u64(&mut self) -> u64 {
        let mut s = self.state;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        self.state = s;
        s
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

impl LuaUserData for LuaRng {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("range", |_, this, (min, max): (f64, f64)| {
            Ok(min + this.next_f64() * (max - min))
        });

        methods.add_method_mut("range_int", |_, this, (min, max): (i64, i64)| {
            let range = (max - min + 1) as u64;
            let val = this.next_u64() % range;
            Ok(min + val as i64)
        });
    }
}

// ===================================================================
// Registration
// ===================================================================

/// Populate `unison.Color`, `unison.Rng`, and `unison.math` on the given `unison` table.
pub fn populate(lua: &Lua, unison: &LuaTable) -> LuaResult<()> {
    // -- unison.Color --
    let color_table = lua.create_table()?;

    color_table.set("hex", lua.create_function(|_, hex: u32| {
        Ok(LuaColor(Color::from_hex(hex)))
    })?)?;

    color_table.set("rgba", lua.create_function(|_, (r, g, b, a): (f32, f32, f32, f32)| {
        Ok(LuaColor(Color::new(r, g, b, a)))
    })?)?;

    unison.set("Color", color_table)?;

    // -- unison.Rng --
    let rng_table = lua.create_table()?;

    rng_table.set("new", lua.create_function(|_, seed: u64| {
        // Ensure non-zero state for xorshift
        let state = if seed == 0 { 1 } else { seed };
        Ok(LuaRng { state })
    })?)?;

    unison.set("Rng", rng_table)?;

    // -- unison.math --
    let math_table = lua.create_table()?;

    math_table.set("lerp", lua.create_function(|_, (a, b, t): (f64, f64, f64)| {
        Ok(a + (b - a) * t)
    })?)?;

    math_table.set("smoothstep", lua.create_function(|_, (edge0, edge1, x): (f64, f64, f64)| {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        Ok(t * t * (3.0 - 2.0 * t))
    })?)?;

    math_table.set("clamp", lua.create_function(|_, (x, min, max): (f64, f64, f64)| {
        Ok(x.clamp(min, max))
    })?)?;

    unison.set("math", math_table)?;

    Ok(())
}

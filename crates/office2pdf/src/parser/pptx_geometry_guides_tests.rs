use super::*;

/// A 400 x 200 box, so `w`, `h`, `ss` and `ls` are all distinguishable.
fn guides() -> GuideList {
    GuideList::new(ShapeExtent::new(400.0, 200.0))
}

fn value(list: &GuideList, name: &str) -> f64 {
    list.resolve(name)
        .unwrap_or_else(|| panic!("{name} did not resolve"))
}

#[test]
fn a_literal_resolves_to_itself() {
    let list = guides();
    assert_eq!(value(&list, "2160"), 2160.0);
    assert_eq!(value(&list, "-90"), -90.0);
    assert_eq!(value(&list, "0"), 0.0);
}

#[test]
fn the_built_in_box_variables_come_from_the_extent() {
    let list = guides();
    assert_eq!(value(&list, "w"), 400.0);
    assert_eq!(value(&list, "h"), 200.0);
    assert_eq!(value(&list, "r"), 400.0);
    assert_eq!(value(&list, "b"), 200.0);
    assert_eq!(value(&list, "l"), 0.0);
    assert_eq!(value(&list, "t"), 0.0);
    assert_eq!(value(&list, "hc"), 200.0);
    assert_eq!(value(&list, "vc"), 100.0);
    assert_eq!(value(&list, "ss"), 200.0, "the short side is the height");
    assert_eq!(value(&list, "ls"), 400.0, "the long side is the width");
}

/// `wd2`, `hd4`, `ssd16` divide a side by the number in the name. The spec
/// enumerates a fixed set, so a handful of each family is the check.
#[test]
fn a_divided_side_reads_its_divisor_from_the_name() {
    let list = guides();
    assert_eq!(value(&list, "wd2"), 200.0);
    assert_eq!(value(&list, "wd10"), 40.0);
    assert_eq!(value(&list, "wd32"), 12.5);
    assert_eq!(value(&list, "hd4"), 50.0);
    assert_eq!(value(&list, "hd8"), 25.0);
    assert_eq!(value(&list, "ssd2"), 100.0);
    assert_eq!(value(&list, "ssd16"), 12.5);
}

/// Angles are in 60000ths of a degree, so a full turn is 21,600,000.
#[test]
fn the_angle_constants_are_fractions_of_a_turn() {
    let list = guides();
    assert_eq!(value(&list, "cd2"), 10_800_000.0, "half a turn");
    assert_eq!(value(&list, "cd4"), 5_400_000.0, "a quarter turn");
    assert_eq!(value(&list, "cd8"), 2_700_000.0);
    assert_eq!(value(&list, "3cd4"), 16_200_000.0, "three quarters");
    assert_eq!(value(&list, "3cd8"), 8_100_000.0);
    assert_eq!(value(&list, "5cd8"), 13_500_000.0);
    assert_eq!(value(&list, "7cd8"), 18_900_000.0);
}

#[test]
fn the_arithmetic_operators_follow_the_spec() {
    let mut list = guides();
    list.define("product", "*/ 3 8 4");
    list.define("sum", "+- 10 5 3");
    list.define("mean", "+/ 10 4 2");
    list.define("magnitude", "mod 2 3 6");
    list.define("root", "sqrt 81");
    list.define("magnitudeless", "abs -12");

    assert_eq!(value(&list, "product"), 6.0, "x * y / z");
    assert_eq!(value(&list, "sum"), 12.0, "x + y - z");
    assert_eq!(value(&list, "mean"), 7.0, "(x + y) / z");
    assert_eq!(value(&list, "magnitude"), 7.0, "sqrt(4 + 9 + 36)");
    assert_eq!(value(&list, "root"), 9.0);
    assert_eq!(value(&list, "magnitudeless"), 12.0);
}

/// `?: x y z` is "if x > 0 then y else z" — the branch a round-tripped
/// geometry uses to pick a sweep direction. Zero takes the else arm.
#[test]
fn the_conditional_tests_strictly_greater_than_zero() {
    let mut list = guides();
    list.define("positive", "?: 1 100 200");
    list.define("zero", "?: 0 100 200");
    list.define("negative", "?: -1 100 200");

    assert_eq!(value(&list, "positive"), 100.0);
    assert_eq!(value(&list, "zero"), 200.0, "zero is not greater than zero");
    assert_eq!(value(&list, "negative"), 200.0);
}

/// `pin x y z` clamps y into [x, z] — how an adjust value is bounded before
/// it becomes a corner radius.
#[test]
fn pin_clamps_the_middle_operand() {
    let mut list = guides();
    list.define("below", "pin 0 -5 10800");
    list.define("within", "pin 0 2160 10800");
    list.define("above", "pin 0 99999 10800");

    assert_eq!(value(&list, "below"), 0.0);
    assert_eq!(value(&list, "within"), 2160.0);
    assert_eq!(value(&list, "above"), 10800.0);
}

#[test]
fn min_and_max_pick_a_side() {
    let mut list = guides();
    list.define("smaller", "min w h");
    list.define("larger", "max w h");

    assert_eq!(value(&list, "smaller"), 200.0);
    assert_eq!(value(&list, "larger"), 400.0);
}

/// The trigonometric operators scale their first operand by the function of
/// the second, which is an angle in 60000ths of a degree.
#[test]
fn the_trigonometric_operators_read_sixty_thousandths_of_a_degree() {
    let mut list = guides();
    list.define("cosine", "cos 100 0");
    list.define("quarter_cosine", "cos 100 5400000");
    list.define("half_cosine", "cos 100 10800000");
    list.define("quarter_sine", "sin 100 5400000");
    list.define("eighth_tangent", "tan 100 2700000");

    assert_eq!(value(&list, "cosine"), 100.0);
    assert!(
        value(&list, "quarter_cosine").abs() < 1e-9,
        "cos 90 deg is 0"
    );
    assert!((value(&list, "half_cosine") + 100.0).abs() < 1e-9);
    assert!((value(&list, "quarter_sine") - 100.0).abs() < 1e-9);
    assert!((value(&list, "eighth_tangent") - 100.0).abs() < 1e-9);
}

/// `at2` answers in the same units, and `cat2`/`sat2` project a length onto
/// the arc-tangent of their remaining two operands.
#[test]
fn the_arc_tangent_operators_answer_in_angle_units() {
    let mut list = guides();
    list.define("quarter", "at2 0 1");
    list.define("projected_x", "cat2 100 3 4");
    list.define("projected_y", "sat2 100 3 4");

    assert!(
        (value(&list, "quarter") - 5_400_000.0).abs() < 1e-6,
        "straight up is a quarter turn, got {}",
        value(&list, "quarter")
    );
    // arctan(4/3) has cosine 3/5 and sine 4/5.
    assert!((value(&list, "projected_x") - 60.0).abs() < 1e-9);
    assert!((value(&list, "projected_y") - 80.0).abs() < 1e-9);
}

/// A formula may name any guide defined before it, which is the whole point
/// of the list: the hundred entries of a round-tripped rounded rectangle
/// build its corner radius out of the adjust value at the head.
#[test]
fn a_guide_may_name_an_earlier_guide() {
    let mut list = guides();
    list.define("f0", "val 2160");
    list.define("f1", "pin 0 f0 10800");
    list.define("f2", "*/ f1 ss 21600");

    assert_eq!(value(&list, "f2"), 2160.0 * 200.0 / 21600.0);
}

/// `val` takes a variable as readily as a number — LibreOffice writes
/// `<a:gd name="f4" fmla="val w"/>` to name the shape width.
#[test]
fn val_accepts_a_variable() {
    let mut list = guides();
    list.define("f4", "val w");
    assert_eq!(value(&list, "f4"), 400.0);
}

/// A name nothing defines is not a number, and inventing zero for it would
/// put a vertex at the shape's corner. The caller drops the point instead.
#[test]
fn an_undefined_name_does_not_resolve() {
    let list = guides();
    assert!(list.resolve("f99").is_none());
    assert!(list.resolve("nonsense").is_none());
}

/// A formula whose operand is undefined, whose operator is unknown, or that
/// divides by zero leaves its guide undefined rather than binding a
/// nonsensical number.
#[test]
fn an_unusable_formula_defines_nothing() {
    let mut list = guides();
    list.define("missing_operand", "*/ f99 1 2");
    list.define("unknown_operator", "frobnicate 1 2");
    list.define("wrong_arity", "*/ 1 2");
    list.define("divided_by_zero", "*/ 1 2 0");

    assert!(list.resolve("missing_operand").is_none());
    assert!(list.resolve("unknown_operator").is_none());
    assert!(list.resolve("wrong_arity").is_none());
    assert!(list.resolve("divided_by_zero").is_none());
}

/// A later `<a:gd>` of the same name wins, so an `<a:avLst>` adjust that the
/// geometry restates is not read twice with two answers.
#[test]
fn a_redefined_guide_takes_its_latest_value() {
    let mut list = guides();
    list.define("f0", "val 2160");
    list.define("f0", "val 5400");
    assert_eq!(value(&list, "f0"), 5400.0);
}

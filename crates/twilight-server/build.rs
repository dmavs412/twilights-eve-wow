//! Stamps the commit this binary was built from, half of the build id a client must match.

fn main() {
    benilla_buildstamp::emit();
}

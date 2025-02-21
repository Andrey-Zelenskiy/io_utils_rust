// Copyright Andrey Zelenskiy, 2024
use crate::config_parse::{Config, FromConfig};

/* ------------------------------------ */
/* Methods for structure initialization */
/* ------------------------------------ */

// Trait for argument structure with required initialization function
pub trait BuilderMethods: Default + FromConfig {
    type Target;

    // Initialize target structure from the parameters
    fn build(&mut self) -> Self::Target;
}

// Trait for initializing a structure from an argument structure
// pub trait TargetFromBuilder<T>
// where
// T: BuilderMethods<Target = Self>,
pub trait TargetFromBuilder {
    type Builder: BuilderMethods<Target = Self>;
    // Initialize new Target from input parameters
    fn builder() -> Self::Builder {
        Self::Builder::default()
    }

    // Initialize Target from a config file
    fn from_config(config: &Config, config_name: &str) -> Self
    where
        Self: Sized,
    {
        //Populate the parameters from the config
        Self::Builder::from_config(config, config_name).build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    pub struct TargetStruct {
        x2: u32,
        xy: u32,
        y2: u32,
    }

    mod test_builder {
        use super::*;

        #[derive(Deserialize, Default)]
        pub struct Builder {
            x: u32,
            y: u32,
        }

        // Add methods for setting values
        impl Builder {
            pub fn set_x(&mut self, x: u32) -> &mut Self {
                self.x = x;
                self
            }

            pub fn set_y(&mut self, y: u32) -> &mut Self {
                self.y = y;
                self
            }
        }

        impl BuilderMethods for Builder {
            type Target = TargetStruct;

            fn build(&mut self) -> Self::Target {
                Self::Target {
                    x2: self.x * self.x,
                    xy: self.x * self.y,
                    y2: self.y * self.y,
                }
            }
        }

        impl TargetFromBuilder for TargetStruct {
            type Builder = Builder;
        }
    }

    #[test]
    fn build() {
        let target = TargetStruct::builder().set_x(1).set_y(2).build();

        assert_eq!(1, target.x2);
        assert_eq!(2, target.xy);
        assert_eq!(4, target.y2);
    }
}

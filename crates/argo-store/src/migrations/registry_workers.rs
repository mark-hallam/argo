//! Ordered migration registrations for versions 1..=20.

use super::{
    v1, v10, v11, v12, v13, v14, v15, v16, v17, v18, v19, v2, v20, v3, v4, v5, v6, v7, v8, v9,
    Migration,
};

pub(crate) fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            up: |tx| Box::pin(v1::up(tx)),
            down: |tx| Box::pin(v1::down(tx)),
        },
        Migration {
            version: 2,
            up: |tx| Box::pin(v2::up(tx)),
            down: |tx| Box::pin(v2::down(tx)),
        },
        Migration {
            version: 3,
            up: |tx| Box::pin(v3::up(tx)),
            down: |tx| Box::pin(v3::down(tx)),
        },
        Migration {
            version: 4,
            up: |tx| Box::pin(v4::up(tx)),
            down: |tx| Box::pin(v4::down(tx)),
        },
        Migration {
            version: 5,
            up: |tx| Box::pin(v5::up(tx)),
            down: |tx| Box::pin(v5::down(tx)),
        },
        Migration {
            version: 6,
            up: |tx| Box::pin(v6::up(tx)),
            down: |tx| Box::pin(v6::down(tx)),
        },
        Migration {
            version: 7,
            up: |tx| Box::pin(v7::up(tx)),
            down: |tx| Box::pin(v7::down(tx)),
        },
        Migration {
            version: 8,
            up: |tx| Box::pin(v8::up(tx)),
            down: |tx| Box::pin(v8::down(tx)),
        },
        Migration {
            version: 9,
            up: |tx| Box::pin(v9::up(tx)),
            down: |tx| Box::pin(v9::down(tx)),
        },
        Migration {
            version: 10,
            up: |tx| Box::pin(v10::up(tx)),
            down: |tx| Box::pin(v10::down(tx)),
        },
        Migration {
            version: 11,
            up: |tx| Box::pin(v11::up(tx)),
            down: |tx| Box::pin(v11::down(tx)),
        },
        Migration {
            version: 12,
            up: |tx| Box::pin(v12::up(tx)),
            down: |tx| Box::pin(v12::down(tx)),
        },
        Migration {
            version: 13,
            up: |tx| Box::pin(v13::up(tx)),
            down: |tx| Box::pin(v13::down(tx)),
        },
        Migration {
            version: 14,
            up: |tx| Box::pin(v14::up(tx)),
            down: |tx| Box::pin(v14::down(tx)),
        },
        Migration {
            version: 15,
            up: |tx| Box::pin(v15::up(tx)),
            down: |tx| Box::pin(v15::down(tx)),
        },
        Migration {
            version: 16,
            up: |tx| Box::pin(v16::up(tx)),
            down: |tx| Box::pin(v16::down(tx)),
        },
        Migration {
            version: 17,
            up: |tx| Box::pin(v17::up(tx)),
            down: |tx| Box::pin(v17::down(tx)),
        },
        Migration {
            version: 18,
            up: |tx| Box::pin(v18::up(tx)),
            down: |tx| Box::pin(v18::down(tx)),
        },
        Migration {
            version: 19,
            up: |tx| Box::pin(v19::up(tx)),
            down: |tx| Box::pin(v19::down(tx)),
        },
        Migration {
            version: 20,
            up: |tx| Box::pin(v20::up(tx)),
            down: |tx| Box::pin(v20::down(tx)),
        },
    ]
}

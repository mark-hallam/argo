//! Ordered migration registrations for versions 41..=56.

use super::{
    v41, v42, v43, v44, v45, v46, v47, v48, v49, v50, v51, v52, v53, v54, v55, v56, Migration,
};

pub(crate) fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 41,
            up: |tx| Box::pin(v41::up(tx)),
            down: |tx| Box::pin(v41::down(tx)),
        },
        Migration {
            version: 42,
            up: |tx| Box::pin(v42::up(tx)),
            down: |tx| Box::pin(v42::down(tx)),
        },
        Migration {
            version: 43,
            up: |tx| Box::pin(v43::up(tx)),
            down: |tx| Box::pin(v43::down(tx)),
        },
        Migration {
            version: 44,
            up: |tx| Box::pin(v44::up(tx)),
            down: |tx| Box::pin(v44::down(tx)),
        },
        Migration {
            version: 45,
            up: |tx| Box::pin(v45::up(tx)),
            down: |tx| Box::pin(v45::down(tx)),
        },
        Migration {
            version: 46,
            up: |tx| Box::pin(v46::up(tx)),
            down: |tx| Box::pin(v46::down(tx)),
        },
        Migration {
            version: 47,
            up: |tx| Box::pin(v47::up(tx)),
            down: |tx| Box::pin(v47::down(tx)),
        },
        Migration {
            version: 48,
            up: |tx| Box::pin(v48::up(tx)),
            down: |tx| Box::pin(v48::down(tx)),
        },
        Migration {
            version: 49,
            up: |tx| Box::pin(v49::up(tx)),
            down: |tx| Box::pin(v49::down(tx)),
        },
        Migration {
            version: 50,
            up: |tx| Box::pin(v50::up(tx)),
            down: |tx| Box::pin(v50::down(tx)),
        },
        Migration {
            version: 51,
            up: |tx| Box::pin(v51::up(tx)),
            down: |tx| Box::pin(v51::down(tx)),
        },
        Migration {
            version: 52,
            up: |tx| Box::pin(v52::up(tx)),
            down: |tx| Box::pin(v52::down(tx)),
        },
        Migration {
            version: 53,
            up: |tx| Box::pin(v53::up(tx)),
            down: |tx| Box::pin(v53::down(tx)),
        },
        Migration {
            version: 54,
            up: |tx| Box::pin(v54::up(tx)),
            down: |tx| Box::pin(v54::down(tx)),
        },
        Migration {
            version: 55,
            up: |tx| Box::pin(v55::up(tx)),
            down: |tx| Box::pin(v55::down(tx)),
        },
        Migration {
            version: 56,
            up: |tx| Box::pin(v56::up(tx)),
            down: |tx| Box::pin(v56::down(tx)),
        },
    ]
}

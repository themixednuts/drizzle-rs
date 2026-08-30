import unittest

from scripts.check_publish import (
    published_internal_dependencies,
    validate_dependency_order,
)


class PublishPreflightTests(unittest.TestCase):
    def test_safe_release_order(self) -> None:
        validate_dependency_order(
            ["types", "core", "driver", "macros"],
            {
                "types": set(),
                "core": {"types"},
                "driver": {"core", "types"},
                "macros": {"driver", "types"},
            },
        )

    def test_versioned_dev_dependency_cannot_point_forward(self) -> None:
        with self.assertRaisesRegex(
            RuntimeError,
            "driver is published before registry dependency macros",
        ):
            validate_dependency_order(
                ["types", "core", "driver", "macros"],
                {
                    "types": set(),
                    "core": {"types"},
                    "driver": {"core", "macros"},
                    "macros": {"driver"},
                },
            )

    def test_internal_dependencies_must_use_exact_workspace_version(self) -> None:
        manifest = {
            "package": {"name": "driver"},
            "dependencies": {"core": {"version": "0.1"}},
        }
        with self.assertRaisesRegex(RuntimeError, "exact workspace pin '=0.1.17'"):
            published_internal_dependencies(manifest, {"core": "0.1.17"})


if __name__ == "__main__":
    unittest.main()

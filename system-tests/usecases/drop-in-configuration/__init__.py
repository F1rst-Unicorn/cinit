#  cinit: process initialisation program for containers
#  Copyright (C) 2019 The cinit developers
#
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU General Public License as published by
#  the Free Software Foundation, either version 3 of the License, or
#  (at your option) any later version.
#
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#  GNU General Public License for more details.
#
#  You should have received a copy of the GNU General Public License
#  along with this program.  If not, see <https://www.gnu.org/licenses/>.

from driver import *


class Test(CinitTest):

    def test(self):
        self.run_cinit(self.get_test_dir(__file__))

        self.assert_on_trace().that(
            Sequential(
                Parallel(
                    Sequential(
                        ChildSpawned("echo-after-1"),
                        ChildExited("echo-after-1")
                    ),
                    Sequential(
                        ChildSpawned("echo-after-2"),
                        ChildExited("echo-after-2")
                    )
                ),
                ChildSpawned("program"),
                ChildExited("program"),
                Parallel(
                    Sequential(
                        ChildSpawned("echo-before-1"),
                        ChildExited("echo-before-1")
                    ),
                    Sequential(
                        ChildSpawned("echo-before-2"),
                        ChildExited("echo-before-2")
                    )
                )
            )
        )


        ChildProcess("program", self)\
            .assert_arg("-o")\
            .assert_arg("system-tests/child-dump/program.yml")\
            .assert_arg("one-two")\
            .assert_arg("three-four")\
            .assert_env("ONE", "one")\
            .assert_env("TWO", "two")\
            .assert_env("THREE", "three")\
            .assert_env("FOUR", "four")\
            .assert_env("ONETWO", "one-two")\
            .assert_env("THREEFOUR", "three-four")\
            .assert_capabilities(["CAP_CHOWN",
                                  "CAP_NET_BIND_SERVICE",
                                  "CAP_DAC_OVERRIDE"])


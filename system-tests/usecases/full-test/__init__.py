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
                ChildSpawned("first"),
                ChildExited("first")
            )
        ).then(
            Parallel(
                Sequential(
                    ChildSpawned("second"),
                    ChildExited("second")
                ),
                Sequential(
                    ChildSpawned("third"),
                    ChildExited("third")
                )
            )
        ).then(
            Sequential(
                ChildSpawned("fourth"),
                ChildExited("fourth")
            )
        )


        ChildProcess("first", self)\
            .assert_arg("-o")\
            .assert_arg("system-tests/child-dump/first.yml")\
            .assert_uid(0)\
            .assert_gid(0)\
            .assert_pty(True)

        ChildProcess("second", self)\
            .assert_uid(1409)\
            .assert_gid(1409)\
            .assert_pty(False)

        ChildProcess("third", self)\
            .assert_uid(1409)\
            .assert_gid(1409)\
            .assert_pty(True)

        ChildProcess("fourth", self)\
            .assert_uid(1409)\
            .assert_gid(1409)\
            .assert_pty(True)

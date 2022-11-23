# dialog_converting.py: converting dialog
#
# Copyright (C) 2022 Hari Rana / TheEvilSkeleton
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: GPL-3.0-only

from gi.repository import Adw, Gtk
from converter.threading import RunAsync
import time


@Gtk.Template(resource_path='/io/gitlab/adhami3310/Converter/gtk/dialog-converting.ui')
class ConvertingDialog(Adw.Window):
    __gtype_name__ = 'ConvertingDialog'

    progressbar = Gtk.Template.Child()

    def __init__(self, parent_window, **kwargs):
        super().__init__(**kwargs)
        self.set_transient_for(parent_window)
        self.pulse = True

        def pulse():
            # Update pulse effect every 0.5 seconds
            while self.pulse:
                time.sleep(.5)
                if self.pulse:
                    self.progressbar.pulse()
        RunAsync(pulse)

    def set_progress(self, progress):
        self.pulse = False
        self.progressbar.set_text(str(progress) + " %")
        self.progressbar.set_fraction(progress / 100)


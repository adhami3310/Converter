# window.py: main window
#
# Copyright (C) 2022 Hari Rana / TheEvilSkeleton
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, version 3.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: GPL-3.0-only


import os
import subprocess
import re
from os.path import basename
from gi.repository import Adw, Gtk, GLib, Gdk
import time
from converter.dialog_converting import ConvertingDialog
from converter.threading import RunAsync
from converter.file_chooser import FileChooser

@Gtk.Template(resource_path='/io/gitlab/adhami3310/Converter/gtk/window.ui')
class ConverterWindow(Adw.ApplicationWindow):
    __gtype_name__ = 'ConverterWindow'

    """ Declare child widgets. """
    toast = Gtk.Template.Child()
    quality = Gtk.Template.Child()
    quality_row = Gtk.Template.Child()
    button_back = Gtk.Template.Child()
    bgcolor = Gtk.Template.Child()
    bgcolor_row = Gtk.Template.Child()
    stack_converter = Gtk.Template.Child()
    button_input = Gtk.Template.Child()
    action_image_size = Gtk.Template.Child()
    filetype = Gtk.Template.Child()
    quality_label = Gtk.Template.Child()
    button_convert = Gtk.Template.Child()
    button_options = Gtk.Template.Child()
    spinner_loading = Gtk.Template.Child()
    image = Gtk.Template.Child()
    # video = Gtk.Template.Child()
    # spin_scale = Gtk.Template.Child()
    button_output = Gtk.Template.Child()
    label_output = Gtk.Template.Child()

    """ Initialize function. """
    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        """ Connect signals. """
        self.button_input.connect('clicked', self.__open_file)
        self.button_convert.connect('clicked', self.__convert)
        self.button_output.connect('clicked', self.__output_location)
        self.button_options.connect('clicked', self.__more_options)
        self.button_back.connect('clicked', self.__less_options)
        self.quality.connect('value-changed', self.__quality_changed)
        self.quality.set_value(92);
        self.bgcolor.set_rgba(Gdk.RGBA(1, 1, 1, 0));
        self.bgcolor.connect('color-set', self.__bg_changed)
        self.filetype.connect('changed', self.filetype_changed)
        # self.spin_scale.connect('value-changed', self.__update_post_convert_image_size)

        """ Declare variables. """
        self.convert_dialog = None
        self.options_window = None

    """ Open file and display it if the user selected it. """
    def __open_file(self, *args):
        FileChooser.open_file(self)

    """ Select output file location. """
    def __output_location(self, *args):
        FileChooser.output_file(self)

    def filetype_changed(self, *args):
        ext = self.filetype.get_text()
        ext = ext[1:] if ext[0:1] == '.' else ext
        self.output_ext = ext
        self.__update_options()
        self.label_output.set_label('(None)')
        self.button_convert.set_sensitive(False)
        self.button_convert.set_has_tooltip(True)

    def __update_options(self):
        self.quality_row.hide()
        self.bgcolor_row.hide()
        inext = self.input_ext
        outext = self.output_ext
        if {'jpg', 'webp', 'jpeg', 'pdf'}.intersection({inext, outext}):
            self.quality_row.show()
        if {'png', 'webp'}.intersection({inext, outext}):
            self.bgcolor_row.show()

    def __quality_changed(self, *args):
        self.quality_label.set_label(str(int(self.quality.get_value())))

    def __more_options(self, *args):
        self.stack_converter.set_visible_child_name('options_page')

    def __less_options(self, *args):
        if self.stack_converter.get_visible_child_name() == 'stack_convert':
            self.stack_converter.set_visible_child_name('stack_welcome_page')
            self.button_back.hide()
        else:
            self.stack_converter.set_visible_child_name('stack_convert')

    def __bg_changed(self, *args):
        color = self.bgcolor.get_rgba()
        print(Gdk.RGBA.to_string(color))

    """ Update progress. """
    def __convert_progress(self, progress):
        if self.convert_dialog:
            self.convert_dialog.set_progress(progress)

    def __convert(self, *args):

        """ Since GTK is not thread safe, prepare some data in the main thread. """
        self.convert_dialog = ConvertingDialog(self)

        """ Run in a separate thread. """
        def run():
            command = ['convert',
                       '-monitor',
                       '-background', f'{Gdk.RGBA.to_string(self.bgcolor.get_rgba())}',
                       self.input_file_path,
                       '-quality', f'{self.quality.get_value()}',
                       self.output_file_path,
                       ]
            process = subprocess.Popen(command, stderr=subprocess.PIPE, universal_newlines=True)
            print('Running: ', end='')
            print(*command)
            """ Read each line, query the percentage and update the progress bar. """
            for line in iter(process.stderr.readline, ''):
                print(line, end='')
                res = re.search('\d\d%', line)
                if res:
                    GLib.idle_add(self.__convert_progress, int(res.group(0)[:-1]))

        """ Run when run() function finishes. """
        def callback(*args):
            self.convert_dialog.close()
            self.convert_dialog = None
            self.converting_completed_dialog()

        """ Run functions asynchronously. """
        RunAsync(run, callback)
        self.convert_dialog.present()

    """ Ask the user if they want to open the file. """
    def converting_completed_dialog(self, *args):
        def response(_widget):
            path = f'file://{self.output_file_path}'
            Gtk.show_uri(self, path, Gdk.CURRENT_TIME)

        toast = Adw.Toast.new(_('Image convertd'))
        toast.set_button_label(_('Open'))
        toast.connect('button-clicked', response)
        self.toast.add_toast(toast)


    """ Update post-convert image size as the user adjusts the spinner. """
    # def __update_post_convert_image_size(self, *args):
    #     convert_image_size = [
    #         self.image_size[1] * int(self.spin_scale.get_value()),
    #         self.image_size[2] * int(self.spin_scale.get_value()),
    #     ]
    #     self.action_convert_image_size.set_subtitle(f'{convert_image_size[0]} × {convert_image_size[1]}')

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
from gi.repository import Adw, Gtk, GLib, Gdk, Gio
import time
from converter.dialog_converting import ConvertingDialog
from converter.threading import RunAsync
from converter.file_chooser import FileChooser
from converter.filters import output_image_extensions, popular_output_image_extensions


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
    resize_row = Gtk.Template.Child()
    stack_converter = Gtk.Template.Child()
    button_input = Gtk.Template.Child()
    action_image_size = Gtk.Template.Child()
    action_image_type = Gtk.Template.Child()
    filetype = Gtk.Template.Child()
    filters = Gtk.Template.Child()
    quality_label = Gtk.Template.Child()
    button_convert = Gtk.Template.Child()
    button_options = Gtk.Template.Child()
    spinner_loading = Gtk.Template.Child()
    image = Gtk.Template.Child()
    supported_output_datatypes = Gtk.Template.Child()
    button_output = Gtk.Template.Child()
    label_output = Gtk.Template.Child()
    resize_filter = Gtk.Template.Child()
    resize_type = Gtk.Template.Child()
    resize_width = Gtk.Template.Child()
    resize_height = Gtk.Template.Child()
    resize_width_value = Gtk.Template.Child()
    resize_height_value = Gtk.Template.Child()
    resize_minmax_width = Gtk.Template.Child()
    resize_minmax_height = Gtk.Template.Child()
    resize_minmax_width_value = Gtk.Template.Child()
    resize_minmax_height_value = Gtk.Template.Child()
    resize_scale_width = Gtk.Template.Child()
    resize_scale_width_value = Gtk.Template.Child()
    resize_scale_height = Gtk.Template.Child()
    resize_scale_height_value = Gtk.Template.Child()
    ratio_width = Gtk.Template.Child()
    ratio_height = Gtk.Template.Child()
    image_container = Gtk.Template.Child()
    ratio_width_value = Gtk.Template.Child()
    ratio_height_value = Gtk.Template.Child()
    resize_filters = ['Point', 'Quadratic', 'Cubic', 'Mitchell', 'Gaussian', 'Lanczos']

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
        self.quality.set_value(92)
        self.bgcolor.connect('color-set', self.__bg_changed)
        self.settings = Gio.Settings("io.gitlab.adhami3310.Converter")
        self.update_output_datatype()
        self.settings.connect("changed::show-less-popular", self.update_output_datatype)
        self.filetype.connect('notify::selected', self.filetype_changed)
        self.resize_row.connect('notify::expanded', self.__update_resize)
        self.resize_type.connect('notify::selected', self.__update_resize)
        self.resize_width.connect('notify::selected', self.__update_resize)
        self.resize_height.connect('notify::selected', self.__update_resize)
        # self.spin_scale.connect('value-changed', self.__update_post_convert_image_size)

        for resize_filter in self.resize_filters:
            self.filters.append(resize_filter)

        """ Declare variables. """
        self.convert_dialog = None
        self.options_window = None

    """ Open file and display it if the user selected it. """
    def __open_file(self, *args):
        FileChooser.open_file(self)

    """ Select output file location. """
    def __output_location(self, *args):
        FileChooser.output_file(self)

    def toggle_datatype(self, *args):
        show_less_popular = self.settings.get_boolean("show-less-popular")
        self.settings.set_boolean("show-less-popular", not show_less_popular)
        self.update_output_datatype()

    def update_output_datatype(self, *args):
        if self.settings.get_boolean("show-less-popular"):
            self.supported_output_datatypes.splice(0, len(self.supported_output_datatypes))
            for supported_file_type in output_image_extensions:
                self.supported_output_datatypes.append(supported_file_type)
            self.filetype.set_selected(output_image_extensions.index("pdf"))
        else:
            self.supported_output_datatypes.splice(0, len(self.supported_output_datatypes))
            for supported_file_type in popular_output_image_extensions:
                self.supported_output_datatypes.append(supported_file_type)
            self.filetype.set_selected(popular_output_image_extensions.index("pdf"))
        self.output_ext = 'pdf'

    def filetype_changed(self, *args):
        ext = self.supported_output_datatypes.get_string(self.filetype.get_selected())
        self.output_ext = ext
        self.__update_options()
        self.label_output.set_label('(None)')
        self.button_convert.set_sensitive(False)
        self.button_convert.set_has_tooltip(True)

    def __update_options(self):
        self.quality_row.hide()
        self.bgcolor_row.hide()
        self.resize_row.hide()
        self.resize_row.set_enable_expansion(False)
        inext = self.input_ext
        outext = self.output_ext
        if {'jpg', 'webp', 'jpeg', 'pdf'}.intersection({inext, outext}):
            self.quality_row.show()
        if {'png', 'webp', 'svg'}.intersection({inext, outext}):
            self.bgcolor_row.show()
            bgcolor = Gdk.RGBA()
            self.bgcolor.set_use_alpha(True)
            bgcolor.parse("#00000000")
            self.bgcolor.set_rgba(bgcolor)
            if outext in {'jpg', 'jpeg'}:
                self.bgcolor.set_use_alpha(False)
                bgcolor.parse("#FFF")
                self.bgcolor.set_rgba(bgcolor)
        self.resize_row.show()
    def __update_resize(self, *args):
        resize_type = self.resize_type.get_selected()
        self.resize_width.hide()
        self.resize_height.hide()
        self.resize_scale_width.hide()
        self.resize_scale_height.hide()
        self.ratio_height.hide()
        self.ratio_width.hide()
        self.resize_minmax_width.hide()
        self.resize_minmax_height.hide()
        if resize_type == 0:
            self.resize_scale_width.show()
            self.resize_scale_height.show()
        elif resize_type == 4:
            self.ratio_height.show()
            self.ratio_width.show()
        elif resize_type == 1:
            self.resize_height.show()
            self.resize_width.show()
            if self.resize_width.get_selected() == 0:
                self.resize_width_value.show()
            else:
                self.resize_width_value.hide()
            if self.resize_height.get_selected() == 0:
                self.resize_height_value.show()
            else:
                self.resize_height_value.hide()
        else:
            self.resize_minmax_width.show()
            self.resize_minmax_height.show()

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

    def __get_resized_commands(self):
        if not self.resize_row.get_expanded(): return []
        resize_filter = self.resize_filters[self.resize_filter.get_selected()]
        resize_type = self.resize_type.get_selected()
        if resize_type == 0:
            def add_per(s):
                if s[-1] == '%': return s
                return s+'%'
            return ['-filter', resize_filter, '-resize', add_per(self.resize_scale_width_value.get_text())+'x'+add_per(self.resize_scale_height_value.get_text())]
        elif resize_type == 4:
            return ['-filter', resize_filter, '-resize', self.ratio_width_value.get_text()+":"+self.ratio_height_value.get_text()]
        elif resize_type == 1:
            if self.resize_width.get_selected() == 0 and self.resize_height.get_selected() == 0:
                return ['-filter', resize_filter, '-resize', self.resize_width_value.get_text()+'x'+self.resize_height_value.get_text()+'!']
            elif self.resize_width.get_selected() == 0:
                return ['-filter', resize_filter, '-resize', self.resize_width_value.get_text()]
            elif self.resize_height.get_selected() == 0:
                return ['-filter', resize_filter, '-resize', 'x'+self.resize_height_value.get_text()]
            else:
                return []
        elif resize_type == 3:
            return ['-filter', resize_filter, '-resize', self.resize_minmax_width_value.get_text()+'x'+self.resize_minmax_height_value.get_text()+'^']
        elif resize_type == 2:
            return ['-filter', resize_filter, '-resize', self.resize_minmax_width_value.get_text()+'x'+self.resize_minmax_height_value.get_text()]
        return []

    def __convert(self, *args):


        """ Since GTK is not thread safe, prepare some data in the main thread. """
        self.convert_dialog = ConvertingDialog(self)

        inp = None

        """ Run in a separate thread. """
        def run():
            command = ['magick',
                      '-monitor',
                       '-background', f'{Gdk.RGBA.to_string(self.bgcolor.get_rgba())}',
                       inp if inp else self.input_file_path,
                       '-flatten',
                       '-quality',
                       f'{self.quality.get_value()}'
                       ]+self.__get_resized_commands()+[
                       self.output_file_path
                       ]
#            command = ['magick', '-version']
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
            file = open(self.output_file_path, "r")
            fid = file.fileno()
            connection = Gio.bus_get_sync(Gio.BusType.SESSION, None)
            proxy = Gio.DBusProxy.new_sync(connection,
                                            Gio.DBusProxyFlags.NONE,
                                            None,
                                            "org.freedesktop.portal.Desktop",
                                            "/org/freedesktop/portal/desktop",
                                            "org.freedesktop.portal.OpenURI",
                                            None)
            try:
                proxy.call_with_unix_fd_list_sync("OpenFile", GLib.Variant("(sha{sv})",("",0,{"ask": GLib.Variant("b", True)})), Gio.DBusCallFlags.NONE, -1, Gio.UnixFDList.new_from_array([fid]), None)
            except Exception as e:
                print("Error: %s\n" % str(e))

        toast = Adw.Toast.new(_('Image converted'))
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

# window.py: main window
#
# Copyright (C) 2022 Khaleel Al-Adhami / adhami3310
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
from os.path import basename, splitext, dirname
import gi
from gi.repository import Adw, Gtk, GLib, Gdk, Gio
import time
from converter.dialog_converting import ConvertingDialog
from converter.threading import RunAsync
from converter.file_chooser import FileChooser
import converter.filters

RESIZE_QUALITY = 92

class ConversionFailed(Exception):
    """Raised when ImageMagick fails. """
    def __init__(self, result_code, output):
        super().__init__()
        self.result_code = result_code
        self.output = output

    def __str__(self):
        return f'Conversion failed.\nResult code: {self.result_code}\nOutput: {self.output}'

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
    svg_size_row = Gtk.Template.Child()
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
    svg_size_width = Gtk.Template.Child()
    svg_size_height = Gtk.Template.Child()
    svg_size_width_value = Gtk.Template.Child()
    svg_size_height_value = Gtk.Template.Child()
    resize_filter = Gtk.Template.Child()
    resize_type = Gtk.Template.Child()
    svg_size_type = Gtk.Template.Child()
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
    invalid_image = Gtk.Template.Child()
    resize_filters = ['Point', 'Quadratic', 'Cubic', 'Mitchell', 'Gaussian', 'Lanczos']

    """ Initialize function. """
    def __init__(self, **kwargs):
        super().__init__(**kwargs)

        self.settings = Gio.Settings('io.gitlab.adhami3310.Converter')
        self.update_output_datatype()

        """ Connect signals. """
        self.button_input.connect('clicked', self.open_file)
        self.button_convert.connect('clicked', self.__output_location)
        self.button_options.connect('clicked', self.__more_options)
        self.button_back.connect('clicked', self.__go_back)
        self.quality.connect('value-changed', self.__quality_changed)
        self.settings.connect('changed::show-less-popular', self.update_output_datatype)
        self.filetype.connect('notify::selected', self.__filetype_changed)
        self.resize_row.connect('notify::expanded', self.__update_resize)
        self.resize_type.connect('notify::selected', self.__update_resize)
        self.resize_width.connect('notify::selected', self.__update_resize)
        self.resize_height.connect('notify::selected', self.__update_resize)
        self.svg_size_row.connect('notify::expanded', self.__update_size)
        self.svg_size_type.connect('notify::selected', self.__update_size)

        for resize_filter in self.resize_filters:
            self.filters.append(resize_filter)

        """ Declare variables. """
        self.convert_dialog = None
        self.options_window = None

    def __on_file_open(self, input_file_path, pixbuf):
        """ Set variables. """
        self.input_file_path = input_file_path
        self.input_ext = basename(splitext(self.input_file_path)[1])[1:]
        self.action_image_type.set_subtitle(f'{self.input_ext.upper()} ({converter.filters.extention_to_mime[self.input_ext.lower()]})')
        self.image_size = (pixbuf.get_width(), pixbuf.get_height())

        """ Display image. """
        self.action_image_size.set_subtitle(f'{self.image_size[0]} × {self.image_size[1]}')
        self.image.set_pixbuf(pixbuf)

        """ Reset widgets. """
        self.resize_scale_height_value.set_text("100")
        self.resize_scale_width_value.set_text("100")
        self.ratio_width_value.set_text("1")
        self.ratio_height_value.set_text("1")
        self.resize_width_value.set_text(str(self.image_size[0]))
        self.resize_height_value.set_text(str(self.image_size[1]))
        self.svg_size_width_value.set_text(str(self.image_size[0]))
        self.svg_size_height_value.set_text(str(self.image_size[1]))
        self.resize_minmax_width_value.set_text(str(self.image_size[0]))
        self.resize_minmax_height_value.set_text(str(self.image_size[1]))
        self.__filetype_changed()
        self.stack_converter.set_visible_child_name('stack_convert')
        self.button_back.show()

    def __on_file_open_error(self, error):
        if error:
            self.stack_converter.set_visible_child_name('stack_invalid_image')
            self.invalid_image.set_description(str(error))
        else:
            self.stack_converter.set_visible_child_name('stack_welcome_page')

    """ Open file and display it. """
    def load_file(self, file_path):
        self.stack_converter.set_visible_child_name('stack_loading')
        self.spinner_loading.start()
        file = Gio.File.new_for_path(file_path)
        FileChooser.load_file(file,
                              self.__on_file_open,
                              self.__on_file_open_error)

        """ Open a file chooser and load the file. """
    def open_file(self, *args):
        self.stack_converter.set_visible_child_name('stack_loading')
        self.spinner_loading.start()
        FileChooser.open_file(self,
                              self.__on_file_open,
                              self.__on_file_open_error)

    """ Select output file location. """
    def __output_location(self, *args):
        def good(output_file_path):
            """ Set variables. """
            self.output_file_path = output_file_path
            self.__convert()

        def bad(message):
            if message:
                self.toast.add_toast(Adw.Toast.new(message))

        base_path = basename(splitext(self.input_file_path)[0])
        directory = dirname(self.input_file_path)
        if not directory.startswith("/home"):
            directory = None
        FileChooser.output_file(self,
                                f'{base_path}.{self.output_ext}',
                                self.output_ext,
                                directory,
                                good,
                                bad)

    """Toggle visibility of less popular datatypes"""
    def toggle_datatype(self, *args):
        show_less_popular = self.settings.get_boolean("show-less-popular")
        self.settings.set_boolean("show-less-popular", not show_less_popular)
        self.update_output_datatype()

    """Update list of output datatypes"""
    def update_output_datatype(self, *args):
        if self.settings.get_boolean('show-less-popular'):
            self.supported_output_datatypes.splice(0, len(self.supported_output_datatypes))
            for supported_file_type in converter.filters.supported_output_formats:
                self.supported_output_datatypes.append(supported_file_type)
            self.filetype.set_selected(converter.filters.supported_output_formats.index('pdf'))
        else:
            self.supported_output_datatypes.splice(0, len(self.supported_output_datatypes))
            for supported_file_type in converter.filters.popular_supported_output_formats:
                self.supported_output_datatypes.append(supported_file_type)
            self.filetype.set_selected(converter.filters.popular_supported_output_formats.index('pdf'))
        self.output_ext = 'pdf'

    """Selected output filetype changed"""
    def __filetype_changed(self, *args):
        ext = self.supported_output_datatypes.get_string(self.filetype.get_selected())
        self.output_ext = ext
        self.__update_options()

    """Updates visible options"""
    def __update_options(self):

        """Hida all options"""
        self.quality_row.hide()
        self.bgcolor_row.hide()
        self.resize_row.hide()
        self.resize_row.set_enable_expansion(False)
        self.svg_size_row.hide()
        self.svg_size_row.set_enable_expansion(False)

        inext = self.input_ext
        outext = self.output_ext

        """Datatypes that can have compression"""
        if {'jpg', 'webp', 'jpeg', 'heif', 'heic', 'avif', 'jxl'}.intersection({inext, outext}):
            self.quality.set_value(RESIZE_QUALITY)
            self.quality_row.show()

        """Datatypes with an alpha layer"""
        if inext in {'png', 'webp', 'svg', 'heic', 'heif', 'avif', 'jxl'}:
            self.bgcolor_row.show()

            self.bgcolor.set_use_alpha(True)
            bgcolor = Gdk.RGBA()
            bgcolor.parse('#00000000')
            self.bgcolor.set_rgba(bgcolor)

            """Datatypes with no alpha layer"""
            if outext in {'jpg', 'jpeg', 'pdf', 'bmp'}:
                self.bgcolor.set_use_alpha(False)
                bgcolor = Gdk.RGBA()
                bgcolor.parse('#FFFFFF')
                self.bgcolor.set_rgba(bgcolor)

        """SVG scaling option"""
        if inext == 'svg':
            self.svg_size_row.show()

        self.resize_row.show()

    """Updates visible resize options"""
    def __update_resize(self, *args):
        resize_type = self.resize_type.get_selected()

        """Hide all resize options"""
        self.resize_width.hide()
        self.resize_height.hide()
        self.resize_scale_width.hide()
        self.resize_scale_height.hide()
        self.ratio_height.hide()
        self.ratio_width.hide()
        self.resize_minmax_width.hide()
        self.resize_minmax_height.hide()

        """Show relevant resize options"""
        if resize_type == 0: #percentage
            self.resize_scale_width.show()
            self.resize_scale_height.show()
        elif resize_type == 1: #exact
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
        elif resize_type == 2 or resize_type == 3: #min or max
            self.resize_minmax_width.show()
            self.resize_minmax_height.show()
        elif resize_type == 4: #ratio
            self.ratio_height.show()
            self.ratio_width.show()

    """Update scaling width vs height"""
    def __update_size(self, *args):
        if self.svg_size_type.get_selected() == 0: #width
            self.svg_size_width.show()
            self.svg_size_height.hide()
        else: #height
            self.svg_size_height.show()
            self.svg_size_width.hide()

    """Update label showing quality"""
    def __quality_changed(self, *args):
        self.quality_label.set_label(str(int(self.quality.get_value())))

    """Press more options"""
    def __more_options(self, *args):
        self.stack_converter.set_visible_child_name('options_page')

    """Pressed the back button"""
    def __go_back(self, *args):
        if self.stack_converter.get_visible_child_name() == 'stack_convert':
            """On Converting Stack"""
            self.stack_converter.set_visible_child_name('stack_welcome_page')
            self.button_back.hide()
        else:
            """On More Options"""
            self.stack_converter.set_visible_child_name('stack_convert')

    """ Update progress """
    def __convert_progress(self, progress):
        if self.convert_dialog:
            self.convert_dialog.set_progress(progress)

    """Get resize commands of ImageMagick"""
    def __get_resized_commands(self):
        if not self.resize_row.get_expanded(): return []
        resize_filter = self.resize_filters[self.resize_filter.get_selected()]
        resize_type = self.resize_type.get_selected()
        if resize_type == 0: #percentage
            def add_per(s):
                return s if s[-1] == '%' else s+'%'
            return ['-filter', resize_filter, '-resize', add_per(self.resize_scale_width_value.get_text())+'x'+add_per(self.resize_scale_height_value.get_text())]
        if resize_type == 1: #exact
            if self.resize_width.get_selected() == 0 and self.resize_height.get_selected() == 0:
                return ['-filter', resize_filter, '-resize', self.resize_width_value.get_text()+'x'+self.resize_height_value.get_text()+'!']
            elif self.resize_width.get_selected() == 0:
                return ['-filter', resize_filter, '-resize', self.resize_width_value.get_text()]
            elif self.resize_height.get_selected() == 0:
                return ['-filter', resize_filter, '-resize', 'x'+self.resize_height_value.get_text()]
            else:
                return []
        if resize_type == 2: #min
            return ['-filter', resize_filter, '-resize', self.resize_minmax_width_value.get_text()+'x'+self.resize_minmax_height_value.get_text()]
        if resize_type == 3: #max
            return ['-filter', resize_filter, '-resize', self.resize_minmax_width_value.get_text()+'x'+self.resize_minmax_height_value.get_text()+'^']
        if resize_type == 4: #ratio
            return ['-filter', resize_filter, '-resize', self.ratio_width_value.get_text()+':'+self.ratio_height_value.get_text()]
        return []

    """Get SVG scaling as ImageMagick command"""
    def __get_sized_commands(self):
        if not self.svg_size_row.get_expanded(): return []
        if self.svg_size_type.get_selected() == 0:
            return ['-size', self.svg_size_width_value.get_text()]
        else:
            return ['-size', 'x'+self.svg_size_height_value.get_text()]

    """Converts the input file to the output file using CLI"""
    def __convert(self, *args):

        """ Since GTK is not thread safe, prepare some data in the main thread. """
        self.convert_dialog = ConvertingDialog(self)
        inp = None #overwrites input_file_path
        out = None #overwrites output_file_path
        """ Run in a separate thread. """
        def run():
            command = ['magick',
                      '-monitor',
                       '-background', f'{Gdk.RGBA.to_string(self.bgcolor.get_rgba())}'
                       ]+self.__get_sized_commands()+[
                       inp if inp else self.input_file_path,
                       '-flatten',
                       '-quality',
                       f'{self.quality.get_value()}'
                       ]+self.__get_resized_commands()+[
                       out if out else self.output_file_path]
#            command = ['magick', 'identify', '-list', 'format']
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
        if self.input_ext == 'SVG' and self.output_ext in {'HEIF', 'HEIC'}:
            out = 'temp.png'
            def convert_to_temp_callback():
                inp = 'temp.png'
                out = None
                RunAsync(run, callback)
            RunAsync(run, convert_to_temp_callback)
        else:
            RunAsync(run, callback)
        self.convert_dialog.present()


    """ Ask the user if they want to open the file. """
    def converting_completed_dialog(self, *args):
        def response(_widget):
            def show_uri(handle, fid):
                connection = Gio.bus_get_sync(Gio.BusType.SESSION, None)
                proxy = Gio.DBusProxy.new_sync(connection,
                                               Gio.DBusProxyFlags.NONE,
                                               None,
                                               'org.freedesktop.portal.Desktop',
                                               '/org/freedesktop/portal/desktop',
                                               'org.freedesktop.portal.OpenURI',
                                                None)
                try:
                    print(handle)
                    proxy.call_with_unix_fd_list_sync('OpenFile',
                                                      GLib.Variant('(sha{sv})', (handle, 0, {'ask': GLib.Variant('b', True)})),
                                                      Gio.DBusCallFlags.NONE,
                                                      -1,
                                                      Gio.UnixFDList.new_from_array([fid]),
                                                      None)
                except Exception as e:
                    print(f'Error: {e}')
            output_file = open(self.output_file_path, 'r')
            fid = output_file.fileno()
            show_uri('', fid)
        toast = Adw.Toast.new(_('Image converted'))
        toast.set_button_label(_('Open'))
        toast.connect('button-clicked', response)
        self.toast.add_toast(toast)


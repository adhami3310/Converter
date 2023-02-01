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
from gi.repository import Adw, Gtk, GLib, Gdk, Gio, Pango, GdkPixbuf
from gettext import gettext as _
from sys import exit
import time
from converter.threading import RunAsync
from converter.file_chooser import FileChooser
import converter.filters

RESIZE_QUALITY = 92
THUMBNAIL_MAX = 3

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
    drop_overlay = Gtk.Template.Child()
    button_cancel = Gtk.Template.Child()
    progressbar = Gtk.Template.Child()
    compression = Gtk.Template.Child()
    dpi = Gtk.Template.Child()
    dpi_value = Gtk.Template.Child()
    supported_compression = Gtk.Template.Child()
    content = Gdk.ContentFormats.new_for_gtype(Gdk.FileList)
    target = Gtk.DropTarget(formats=content, actions=Gdk.DragAction.COPY)
    resize_filters = ['Point', 'Quadratic', 'Cubic', 'Mitchell', 'Gaussian', 'Lanczos']

    style_provider = Gtk.CssProvider()

    """ Initialize function. """
    def __init__(self, **kwargs):
        super().__init__(**kwargs)

        """ Declare variables. """
        self.options_window = None
        self.input_file_paths = []
        self.collection = False
        self.settings = Gio.Settings('io.gitlab.adhami3310.Converter')

        """ Connect signals. """
        self.button_input.connect('clicked', self.open_file)
        self.button_convert.connect('clicked', self.__output_location)
        self.button_options.connect('clicked', self.__more_options)
        self.button_back.connect('clicked', self.__go_back)
        self.quality.connect('value-changed', self.__quality_changed)
        self.settings.connect('changed::show-less-popular', self.update_output_datatype)
        self.filetype.connect('notify::selected', self.__filetype_changed)
        self.compression.connect('notify::selected', self.__compression_changed)
        self.resize_row.connect('notify::expanded', self.__update_resize)
        self.resize_type.connect('notify::selected', self.__update_resize)
        self.resize_width.connect('notify::selected', self.__update_resize)
        self.resize_height.connect('notify::selected', self.__update_resize)
        self.svg_size_row.connect('notify::expanded', self.__update_size)
        self.svg_size_type.connect('notify::selected', self.__update_size)
        self.button_cancel.connect('clicked', self.__cancel)
        self.target.connect('drop', self.__on_drop)
        self.target.connect('enter', self.__on_enter)
        self.target.connect('leave', self.__on_leave)
        self.add_controller(self.target)

        for resize_filter in self.resize_filters:
            self.filters.append(resize_filter)

    """Loads an image from the clipboard"""
    def load_cb(self):
        display = self.get_display()
        cb = display.get_clipboard()
        print(cb.get_formats().get_mime_types())
        if 'image/png' in cb.get_formats().get_mime_types():
            def load_clipboard(_, result, userdata):
                image = cb.read_texture_finish(result)
                image.save_to_png("converted_8533567899.png")
                self.load_file(["converted_8533567899.png"])
            cb.read_texture_async(None, load_clipboard, None)

    def __on_paths_received(self, files, paths):
        self.input_file_paths = paths
        self.collection = False

        def collect_pixbuf(*args):
            self.pixbufs = []
            for file in files[:THUMBNAIL_MAX]:
                file.read_async(GLib.PRIORITY_DEFAULT,
                                    None,
                                    FileChooser.open_file_done,
                                    (self.__recieve_image, self.__on_file_load_error))
        
        def get_first_animated():
            command = ['magick',
                      'identify',
                       paths[0]]
            print('Running: ', end='')
            print(*command)
            self.process = subprocess.Popen(command, stdout=subprocess.PIPE)
            count = len(self.process.stdout.readlines())
            if count > 1:
                self.collection = True

        if len(paths) > 1:
            self.collection = True
            collect_pixbuf()
        else:
            RunAsync(get_first_animated, collect_pixbuf)

    def __recieve_image(self, paths, pixbufs):
        self.pixbufs += pixbufs
        if len(self.pixbufs) >= min(THUMBNAIL_MAX, len(self.input_file_paths)):
            self.__on_file_open()

    def __on_file_open(self):
        self.compression.hide()
        self.action_image_size.hide()

        self.pixbufs = [p for p in self.pixbufs if p is not None]

        """ Set variables. """
        if self.collection:
            self.compression.show()
        self.input_exts = [splitext(file_path)[1][1:].upper() for file_path in self.input_file_paths]
        self.action_image_type.set_subtitle(", ".join(set([f'{ext.upper()} ({converter.filters.extention_to_mime[ext.lower()]})' for ext in self.input_exts])))
        """ Display image. """
        if len(self.pixbufs) == 0:
            self.image_size = (1000, 1000)
        elif len(self.input_file_paths) == 1:
            self.image_size = (self.pixbufs[0].get_width(), self.pixbufs[0].get_height())
            self.image_container.show()
            self.action_image_size.set_subtitle(f'{self.image_size[0]} × {self.image_size[1]}')
            self.action_image_size.show()
            self.image.set_pixbuf(self.pixbufs[0])
        else:
            def stack_images(p):
                side = min(min([q.get_width() for q in p]), min([q.get_height() for q in p]))
                overlap = 0.2
                result = GdkPixbuf.Pixbuf.new(0, True, 8, side + (len(p)-1)*overlap*side, side + (len(p)-1)*overlap*side)
                for i, q in enumerate(p):
                    q.scale(result, i*overlap*side, i*overlap*side, side, side, i*overlap*side, i*overlap*side, max(side/q.props.width, side/q.props.height), max(side/q.props.width, side/q.props.height), 2)
                return result
            self.image_size = (self.pixbufs[0].get_width(), self.pixbufs[0].get_height())
            self.image_container.show()
            self.image.set_pixbuf(stack_images(self.pixbufs))

        """ Reset widgets. """
        self.quality.set_value(RESIZE_QUALITY)
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
        self.dpi_value.set_text("300")
        self.__compression_changed()
        self.update_output_datatype()
        self.__filetype_changed()
        self.stack_converter.set_visible_child_name('stack_convert')

    def __on_file_load_error(self, error):
        self.__recieve_image(None, [None])

    def __on_file_open_error(self, error):
        if error:
            self.stack_converter.set_visible_child_name('stack_invalid_image')
        elif not self.input_file_paths:
            self.stack_converter.set_visible_child_name('stack_welcome_page')
        else:
            self.stack_converter.set_visible_child_name('stack_convert')

    def __on_file_start(self):
        self.button_back.hide()
        self.stack_converter.set_visible_child_name('stack_loading')
        self.spinner_loading.start()

    """ Open file and display it. """
    def load_file(self, file_paths):
        if len(file_paths) == len(self.input_file_paths) == 1 and file_paths[0] == self.input_file_paths[0]: return
        self.input_file_paths = []
        files = [Gio.File.new_for_path(fp) for fp in file_paths]
        FileChooser.load_file(files,
                              self.__on_file_start,
                              self.__on_paths_received,
                              self.__on_file_open_error)

    """ Open gfile and display it. """
    def load_gfile(self, gfiles):
        gfiles = gfiles.get_files()
        if len(gfiles) == len(self.input_file_paths) == 1 and gfiles[0].get_path() == self.input_file_paths[0]: return
        self.input_file_paths = []
        FileChooser.load_file(gfiles,
                              self.__on_file_start,
                              self.__on_paths_received,
                              self.__on_file_open_error)

    """ Open a file chooser and load the file. """
    def open_file(self, *args):
        FileChooser.open_file(self,
                              self.input_file_paths,
                              self.__on_file_start,
                              self.__on_paths_received,
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

        base_path = basename(splitext(self.input_file_paths[0])[0])
        directory = dirname(self.input_file_paths[0])
        if not directory.startswith("/home"):
            directory = None
        if self.collection:
            ext = self.compression_ext
        else:
            ext = self.output_ext.lower()
        FileChooser.output_file(self,
                                f'{base_path}.{ext}',
                                ext,
                                directory,
                                good,
                                bad)

    def __on_drop(self, _, file: Gio.File, *args):
        self.load_gfile(file)

    def __on_enter(self,*args):
        self.previous_stack = self.stack_converter.get_visible_child_name()
        self.stack_converter.set_visible_child_name('stack_drop')
        return Gdk.DragAction.COPY

    def __on_leave(self, *args):
        self.stack_converter.set_visible_child_name(self.previous_stack)

    """Toggle visibility of less popular datatypes"""
    def toggle_datatype(self, *args):
        show_less_popular = self.settings.get_boolean("show-less-popular")
        self.settings.set_boolean("show-less-popular", not show_less_popular)
        self.update_output_datatype()

    """Update list of output datatypes"""
    def update_output_datatype(self, *args):
        if self.collection:
            self.supported_compression.splice(0, len(self.supported_compression))
            for supported_file_type in converter.filters.compressed_formats:
                self.supported_compression.append(supported_file_type)
            self.compression.set_selected(converter.filters.compressed_formats.index('zip'))
        self.compression_ext = 'zip'
        if self.settings.get_boolean('show-less-popular'):
            self.supported_output_datatypes.splice(0, len(self.supported_output_datatypes))
            for supported_file_type in converter.filters.supported_output_formats:
                self.supported_output_datatypes.append(supported_file_type)
            self.filetype.set_selected(converter.filters.supported_output_formats.index('png'))
        else:
            self.supported_output_datatypes.splice(0, len(self.supported_output_datatypes))
            for supported_file_type in converter.filters.popular_supported_output_formats:
                self.supported_output_datatypes.append(supported_file_type)
            self.filetype.set_selected(converter.filters.popular_supported_output_formats.index('png'))
        self.output_ext = 'png'

    """Selected output filetype changed"""
    def __filetype_changed(self, *args):
        ext = self.supported_output_datatypes.get_string(self.filetype.get_selected())
        if ext:
            self.output_ext = ext.upper()
            self.__update_options()

    def __compression_changed(self, *args):
        ext = self.supported_compression.get_string(self.compression.get_selected())
        self.compression_ext = ext

    """Updates visible options"""
    def __update_options(self):
        if self.output_ext is None: return

        """Hida all options"""
        self.quality_row.hide()
        self.resize_row.hide()
        self.resize_row.set_enable_expansion(False)
        self.svg_size_row.hide()
        self.svg_size_row.set_enable_expansion(False)
        self.dpi.hide()

        inext = set([s.lower() for s in self.input_exts])
        outext = self.output_ext.lower()

        """Datatypes that can have compression"""
        if {'jpg', 'webp', 'jpeg', 'heif', 'heic', 'avif', 'jxl'}.intersection(inext | { outext }):
            self.quality_row.show()

        """Datatypes with an alpha layer"""
        if inext.intersection({'svg', 'png', 'webp', 'heic', 'heif', 'avif', 'jxl'}):
            self.bgcolor_row.show()

            """Datatypes with no alpha layer"""
            if outext in {'jpg', 'jpeg', 'pdf', 'bmp'}:
                if self.bgcolor.get_use_alpha() == True:
                    self.bgcolor.set_use_alpha(False)
                    bgcolor = Gdk.RGBA()
                    bgcolor.parse('#FFFFFF')
                    self.bgcolor.set_rgba(bgcolor)
            else:
                if self.bgcolor.get_use_alpha() == False:
                    self.bgcolor.set_use_alpha(True)
                    bgcolor = Gdk.RGBA()
                    bgcolor.parse('#00000000')
                    self.bgcolor.set_rgba(bgcolor)
        else:
            self.bgcolor_row.hide()


        """SVG scaling option"""
        if 'svg' in inext:
            self.svg_size_row.show()

        if 'pdf' in inext:
            self.dpi.show()

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
        self.button_back.show()
        self.stack_converter.set_visible_child_name('options_page')

    """Pressed the back button"""
    def __go_back(self, *args):
        """On More Options"""
        self.button_back.hide()
        self.stack_converter.set_visible_child_name('stack_convert')

    """ Update progress """
    def __convert_progress(self, progress, current=1, count=1):
        if self.stack_converter.get_visible_child_name() == 'stack_converting':
            self.set_progress(progress, current, count)

    def set_progress(self, progress, current, count):
        progress_str = str(progress) if str(progress)[-1] == '%' else str(progress) + '%'
        self.progressbar.set_text(f'{current}/{count}, {progress_str}')
        self.progressbar.set_fraction(progress / 100)

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
        self.cancelled = False

        def reset_widgets():
            self.button_convert.set_sensitive(True)
            self.progressbar.set_text(_('Loading…'))
            self.progressbar.set_fraction(0)
            self.cancelled = False

        def path_to_basename(file_path):
            return basename(splitext(file_path)[0])

        def path_to_ext(file_path):
            return splitext(file_path)[1][1:]


        """ Run in a separate thread. """
        def convert_direct(input_file, output_file, current, count):
            ext = basename(splitext(input_file)[1])[1:].upper()
            print('converting ', input_file, ' to ', output_file)
            command = ['magick',
                      '-monitor'
                       ]+(self.__get_sized_commands() if ext == 'SVG' else []) +[
                       '-background', f'{Gdk.RGBA.to_string(self.bgcolor.get_rgba())}',
                       input_file,
                       '-quality',
                       f'{self.quality.get_value()}'
                       ]+self.__get_resized_commands()+[
                       output_file]
            print('Running: ', end='')
            print(*command)
            self.process = subprocess.Popen(command, stderr=subprocess.PIPE, universal_newlines=True)
            output = ''
            """ Read each line, query the percentage and update the progress bar. """
            for line in iter(self.process.stderr.readline, ''):
                print(line, end='')
                output += line
                res = re.search('.\d\d%', line)
                if res:
                    GLib.idle_add(self.__convert_progress, int(res.group(0)[:-1]), current, count)
            result = self.process.poll()
            if result != 0 and result != None:
                raise ConversionFailed(result, output)

        """ Run in a separate thread. """
        def convert_coalesce(input_file, output_file, current, count):
            print('converting ', input_file, ' to ', output_file)
            command = ['magick',
                      '-monitor',
                       input_file,
                       '-coalesce',
                       '-fill', f'{Gdk.RGBA.to_string(self.bgcolor.get_rgba())}', '-opaque', 'none',
                       '-quality',
                       f'{self.quality.get_value()}'
                       ]+self.__get_resized_commands()+[
                       output_file]
            print('Running: ', end='')
            print(*command)
            self.process = subprocess.Popen(command, stderr=subprocess.PIPE, universal_newlines=True)
            output = ''
            """ Read each line, query the percentage and update the progress bar. """
            for line in iter(self.process.stderr.readline, ''):
                print(line, end='')
                output += line
                res = re.search('.\d\d%', line)
                if res:
                    GLib.idle_add(self.__convert_progress, int(res.group(0)[:-1]), current, count)
            result = self.process.poll()
            if result != 0 and result != None:
                raise ConversionFailed(result, output)

        """ Run in a separate thread. """
        def convert_pdf(input_file, output_file, current, count):
            print('converting ', input_file, ' to ', output_file)
            command = ['gs',
                       '-sDEVICE=png16m',
                       '-dTextAlphaBits=4',
                       '-o',
                       path_to_basename(output_file)+'%03d.'+path_to_ext(output_file),
                       '-r' + self.dpi_value.get_text(),
                       input_file]
            print('Running: ', end='')
            print(*command)
            self.process = subprocess.Popen(command, stderr=subprocess.PIPE, universal_newlines=True)
            output = ''
            """ Read each line, query the percentage and update the progress bar. """
            for line in iter(self.process.stderr.readline, ''):
                print(line, end='')
                output += line
                res = re.search('.\d\d%', line)
                if res:
                    GLib.idle_add(self.__convert_progress, int(res.group(0)[:-1]), current, count)
            result = self.process.poll()
            if result != 0 and result != None:
                raise ConversionFailed(result, output)
        
        """ Run in a separate thread. """
        def convert_first_frame(input_file, output_file, current, count):
            ext = basename(splitext(input_file)[1])[1:].upper()
            print('converting ', input_file, ' to ', output_file)
            command = ['magick',
                       '-monitor'
                       ]+(self.__get_sized_commands() if ext == 'SVG' else []) +[
                       '-background', f'{Gdk.RGBA.to_string(self.bgcolor.get_rgba())}',
                       input_file+"[0]",
                       '-flatten',
                       '-quality',
                       f'{self.quality.get_value()}'
                       ]+self.__get_resized_commands()+[
                       output_file]
#            command = ['magick', 'identify', '-list', 'format']
            print('Running: ', end='')
            print(*command)
            self.process = subprocess.Popen(command, stderr=subprocess.PIPE, universal_newlines=True)
            output = ''
            """ Read each line, query the percentage and update the progress bar. """
            for line in iter(self.process.stderr.readline, ''):
                print(line, end='')
                output += line
                res = re.search('.\d\d%', line)
                if res:
                    GLib.idle_add(self.__convert_progress, int(res.group(0)[:-1]), current, count)
            result = self.process.poll()
            if result != 0 and result != None:
                raise ConversionFailed(result, output)

        """ Run when run() function finishes. """
        def callback(result, error):
            if self.cancelled == True:
                self.toast.add_toast(Adw.Toast.new(_('Converting Cancelled')))
            else:
                self.converting_completed_dialog(error)

            self.stack_converter.set_visible_child_name('stack_convert')
            reset_widgets()

        input_file_paths = self.input_file_paths
        output_file_path = self.output_file_path

        def convert_individual(input_file_path, output_file_path, current, count, callback):
            """ Run functions asynchronously. """
            ext = splitext(input_file_path)[1][1:].upper()
            if ext == 'SVG' and self.output_ext in {'HEIF', 'HEIC'}:
                def convert_to_temp_callback(res, err):
                    RunAsync(lambda: convert_first_frame("converted_853356789.png", output_file_path, current, count, True), callback)
                RunAsync(lambda: convert_first_frame(input_file_path, "converted_853356789.png", current, count), convert_to_temp_callback)
            elif ext == 'GIF' and self.output_ext == 'WEBP':
                RunAsync(lambda: convert_direct(input_file_path, output_file_path, current, count), callback)
            elif ext == 'WEBP' and self.output_ext == 'GIF':
                RunAsync(lambda: convert_direct(input_file_path, output_file_path, current, count), callback)
            elif ext in {'GIF', 'WEBP'}:
                RunAsync(lambda: convert_coalesce(input_file_path, output_file_path, current, count), callback)
            elif ext == 'ICO':
                RunAsync(lambda: convert_direct(input_file_path, output_file_path, current, count), callback)
            elif ext == 'PDF':
                def convert_to_temp_callback(page_count, result, error):
                    if error:
                        callback(result, None)
                        return
                    page_number = '%03d' % page_count
                    output_path = path_to_basename(output_file_path) + page_number + '.' + path_to_ext(output_file_path)
                    if page_count == 1:
                        output_path = output_file_path
                    RunAsync(lambda: convert_first_frame('converted_853356789' + page_number +'.png',
                                                         output_path,
                                                         current,
                                                         count),
                             lambda res, err: convert_to_temp_callback(page_count+1, res, err))
                RunAsync(lambda: convert_pdf(input_file_path, 'converted_853356789.png', current, count), lambda res, err: convert_to_temp_callback(1, res, err))
            else:
                RunAsync(lambda: convert_first_frame(input_file_path, output_file_path, current, count), callback)

        def cleanupStart(result, error):
            if error:
                callback(result, error)
            RunAsync(cleanup, cleanupCallback)

        def cleanup():
            print('cleaning')
            command = ['find',
                       '.',
                       '-name',
                       'converted_853356789*',
                       '-delete']
            print('Running: ', end='')
            print(*command)
            self.process = subprocess.Popen(command, stderr=subprocess.PIPE, universal_newlines=True)
        
        def cleanupCallback(result, error):
            callback(result, error)

        def convert_group(input_file_paths, output_file_path):

            def input_path_to_output_path(current_input_file_path, i=0):
                basename = path_to_basename(current_input_file_path)
                count = output_basenames[:i].count(basename)
                all_count = output_basenames.count(basename)
                return basename + (f'-{count+1}' if all_count > 1 else '') + '.' + self.output_ext.lower()

            output_basenames = [path_to_basename(path) for path in input_file_paths]
            output_file_paths = [input_path_to_output_path(path, i) for i, path in enumerate(input_file_paths)]

            def convert_individual_callback(i, finalcallback, result, error):
                if error != None or i >= len(input_file_paths) or self.cancelled:
                    finalcallback(result, error)
                    return
                current_input_file_path = input_file_paths[i]
                current_output_file_path = output_file_paths[i]
                convert_individual(current_input_file_path, current_output_file_path, i+1, len(input_file_paths), lambda result, error: convert_individual_callback(i+1, finalcallback, result, error))

            def group_completed(result, error):
                if self.cancelled == True:
                    self.toast.add_toast(Adw.Toast.new(_('Converting Cancelled')))
                    reset_widgets()
                    return
                if error:
                    self.stack_converter.set_visible_child_name('stack_convert')
                    reset_widgets()
                    self.converting_completed_dialog(error)
                    return
                RunAsync(compress, cleanupStart)

            def compress():
                def join_find():
                    result = []
                    for path in output_file_paths:
                        result += ['-name', path_to_basename(path) + '*.' + self.output_ext.lower(), '-o']
                    return result[:-1]
                find_command = ['find', '-maxdepth', '1', '-type', 'f', '('] + join_find() + [')', '-print']
                command = ['zip', '-FSm', output_file_path, '-@']
                find_process = subprocess.Popen(find_command, stdout=subprocess.PIPE)
                self.process = subprocess.Popen(command, stdin=find_process.stdout, stderr=subprocess.PIPE, universal_newlines=True)
                print('Running: ', end='')
                print(*command)
                output = ''
                """ Read each line """
                for line in iter(self.process.stderr.readline, ''):
                    print(line, end='')
                    output += line
                result = self.process.poll()
                if result != 0 and result != None:
                    raise ConversionFailed(result, output)

            convert_individual_callback(0, group_completed, [], None)

        if self.collection:
            convert_group(input_file_paths, output_file_path)
        else:
            convert_individual(input_file_paths[0], output_file_path, 1, 1, cleanupStart)
        self.stack_converter.set_visible_child_name('stack_converting')
        self.button_convert.set_sensitive(False)

    def __cancel(self, *args):
        def function():
            self.cancelled = True
            self.process.kill()
            self.stack_converter.set_visible_child_name('stack_convert')
            self.button_convert.set_sensitive(True)
        self.close_dialog(function)

    """ Prompt the user to close the dialog. """
    def close_dialog(self, function):
        self.stop_converting_dialog = Adw.MessageDialog.new(
            self,
            _('Stop converting?'),
            _('You will lose all progress.'),
        )
        def response(dialog, response_id):
            if response_id == 'stop':
                function()

        self.stop_converting_dialog.add_response('cancel', _('_Cancel'))
        self.stop_converting_dialog.add_response('stop', _('_Stop'))
        self.stop_converting_dialog.set_response_appearance('stop', Adw.ResponseAppearance.DESTRUCTIVE)
        self.stop_converting_dialog.connect('response', response)
        self.stop_converting_dialog.present()


    """ Ask the user if they want to open the file. """
    def converting_completed_dialog(self, error):
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
        toast = None
        if error is None:
            toast = Adw.Toast.new(_('Image converted'))
            toast.set_button_label(_('Open'))
            toast.connect('button-clicked', response)
            self.toast.add_toast(toast)
        else:
            dialog = Adw.MessageDialog.new(self,
                                           _('Error while processing'),
                                           None)
            sw = Gtk.ScrolledWindow()
            sw.set_min_content_height(200)
            sw.set_min_content_width(400)
            sw.add_css_class('card')

            text = Gtk.Label()
            text.set_label(str(error))
            text.set_margin_top(12)
            text.set_margin_bottom(12)
            text.set_margin_start(12)
            text.set_margin_end(12)
            text.set_xalign(0)
            text.set_yalign(0)
            text.add_css_class('monospace')
            text.set_wrap(True)
            text.set_wrap_mode(Pango.WrapMode.WORD_CHAR)

            sw.set_child(text)
            dialog.set_extra_child(sw)

            def error_response(dialog, response_id):
                if response_id == 'copy':
                    clipboard = Gdk.Display.get_default().get_clipboard()
                    clipboard.set(str(error))
                    toast = Adw.Toast.new(_('Error copied to clipboard'))
                    self.toast.add_toast(toast)
                dialog.close()

            dialog.add_response('copy', _('_Copy to clipboard'))
            dialog.set_response_appearance('copy', Adw.ResponseAppearance.SUGGESTED)
            dialog.add_response('ok', _('_Dismiss'))
            dialog.connect('response', error_response)
            dialog.present()

    """ Close dialog. """
    def do_close_request(self):
        if self.stack_converter.get_visible_child_name() == 'stack_converting':
            def function():
                exit()
            self.close_dialog(function)
            return True


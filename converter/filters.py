# filters.py: supported filetypes and filter helpers
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

from gi.repository import Gtk

""" Declare lists. """
supported_input_formats = ['png', 'jpg', 'jpeg', 'webp', 'svg', 'heif', 'heic', 'bmp', 'avif', 'jxl', 'tiff', 'tif', 'pdf', 'gif', 'ico']
supported_output_formats = sorted(['bmp', 'png', 'jpg', 'jpeg', 'webp', 'pdf', 'heic', 'heif', 'avif', 'jxl', 'tif', 'tiff', 'gif', 'ico'])
popular_supported_output_formats = sorted(['bmp', 'png', 'jpg', 'webp', 'pdf', 'heic', 'avif', 'jxl', 'gif'])
compressed_formats = sorted(['zip', 'tar.gz'])

extention_to_mime = {
    'jpg': 'image/jpeg',
    'jpeg': 'image/jpeg',
    'png': 'image/png',
    'pdf': 'application/pdf',
    'webp': 'image/webp',
    'svg': 'image/svg+xml',
    'heic': 'image/heic',
    'heif': 'image/heif',
    'bmp': 'image/bmp',
    'avif': 'image/avif',
    'jxl': 'image/jxl',
    'zip': 'application/zip',
    'tar.gz': 'application/gzip',
    'tiff': 'image/tiff',
    'tif': 'image/tiff',
    'gif': 'image/gif',
    'ico': 'image/x-icon'
}

""" Formats getter function. """
def get_format_filters(type):
    if type == 'input':
        return [extention_to_mime[image] for image in supported_input_formats]
    elif type == "popular_output":
        return [extention_to_mime[image] for image in popular_supported_output_formats]
    elif type == "output":
        return [extention_to_mime[image] for image in supported_output_formats]

def get_file_filter(name, formats):
    filter = Gtk.FileFilter()
    for format in formats:
        filter.add_mime_type(extention_to_mime[format.lower()])
    filter.set_name(name)
    return filter

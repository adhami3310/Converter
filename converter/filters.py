# filters.py: list container getter and setter functions regarding filters
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

from gi.repository import Gtk

""" Declare lists. """
image_formats = ['image/png', 'image/jpeg', 'image/jpg', 'image/webp']
video_formats = ['video/mp4']

""" Formats getter function. """
def get_format_filters(type):
    if type == 'image':
        return image_formats
    elif type == 'video':
        return video_formats
    else:
        return image_formats + video_formats

""" Formats setter function. """
def set_formats(formats):
    filter = Gtk.FileFilter()
    file_extensions = []
    for format in formats:
        filter.add_mime_type(format)

    filter.set_name(_('Supported image files'))
    return filter

""" Supported filters. """
def supported_filters():
    # filter = set_formats(image_formats + video_formats)
    filter = set_formats(image_formats)
    return filter

""" Image specific filters. """
def image_filters():
    filter = set_formats(image_formats)
    return filter

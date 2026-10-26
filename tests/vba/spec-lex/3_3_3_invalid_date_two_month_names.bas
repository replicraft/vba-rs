' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, at most one of the date parts can be a month name
' ERROR: load
Public Sub Main()
    Dim d As Date
    d = #Jan Feb 2003#
End Sub

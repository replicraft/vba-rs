' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, three numbers with no legal month/day interpretation are invalid
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
    Dim d As Date
    d = #13/13/2003#
End Sub

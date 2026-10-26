' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, a day that is not legal for the month makes the date-value invalid
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
    Dim d As Date
    d = #2/30/2003#
End Sub

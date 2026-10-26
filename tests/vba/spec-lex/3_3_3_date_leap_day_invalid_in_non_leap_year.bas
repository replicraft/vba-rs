' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, February 29 is not a legal day in 2003
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
    Dim d As Date
    d = #2/29/2003#
End Sub

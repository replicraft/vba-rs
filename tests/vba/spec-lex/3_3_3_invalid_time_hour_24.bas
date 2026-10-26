' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, hour-value MUST be in 0 to 23
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
    Dim d As Date
    d = #24:00#
End Sub

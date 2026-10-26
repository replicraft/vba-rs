' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, minute-value MUST be in 0 to 59
' ERROR: load
Public Sub Main()
    Dim d As Date
    d = #10:60#
End Sub

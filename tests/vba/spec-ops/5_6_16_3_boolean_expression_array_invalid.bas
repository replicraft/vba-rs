' INV: vbal.5.6.16.3, stmt.while-wend
' SOURCE: spec ms-vbal 5.6.16.3 boolean expression invalid when Let-coercion of its declared type to Boolean is invalid (array)
' ERROR: load
Public Sub Main()
    Dim a(1 To 2) As Long
    While a
    Wend
End Sub

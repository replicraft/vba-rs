' INV: vbal.5.6.9.7, op.is
' SOURCE: spec ms-vbal 5.6.9.7 Is operator operand declared type must be class, Object or Variant
' ERROR: load
Public Sub Main()
    Dim n As Long, o As Object, r As Boolean
    r = (n Is o)
End Sub

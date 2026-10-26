' INV: vbal.5.4.3.1, stmt.dim
' SOURCE: spec ms-vbal 5.4.3.1 variable name must differ from other variables
' ERROR: load
Public Sub Main()
    Dim x As Integer
    Dim x As Long
    Debug.Print "x"
End Sub

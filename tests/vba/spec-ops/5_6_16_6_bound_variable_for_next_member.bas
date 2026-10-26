' INV: vbal.5.6.16.6, stmt.for-next
' SOURCE: spec ms-vbal 5.6.16.6 bound variable expression must be a variable (For counter that is a constant is invalid)
' ERROR: load
Private Const K As Long = 3
Public Sub Main()
    For K = 1 To 2
    Next K
End Sub

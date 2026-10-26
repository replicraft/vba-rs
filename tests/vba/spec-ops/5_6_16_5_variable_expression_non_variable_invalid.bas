' INV: vbal.5.6.16.5, stmt.let
' SOURCE: spec ms-vbal 5.6.16.5 variable expression invalid when classified as something other than a variable
' ERROR: load
Private Const K As Long = 3
Public Sub Main()
    K = 4
End Sub

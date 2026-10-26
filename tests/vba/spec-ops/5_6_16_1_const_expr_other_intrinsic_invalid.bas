' INV: vbal.5.6.16.1
' SOURCE: spec ms-vbal 5.6.16.1 constant expression may only invoke the listed intrinsics (Left is not listed)
' ERROR: load
Private Const C As String = Left("abc", 1)
Public Sub Main()
    Debug.Print C
End Sub

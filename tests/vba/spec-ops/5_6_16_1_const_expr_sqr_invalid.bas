' INV: vbal.5.6.16.1
' SOURCE: spec ms-vbal 5.6.16.1 constant expression may only invoke the listed intrinsics (Sqr is not listed)
' ERROR: load
Private Const C As Double = Sqr(16)
Public Sub Main()
    Debug.Print CStr(C)
End Sub

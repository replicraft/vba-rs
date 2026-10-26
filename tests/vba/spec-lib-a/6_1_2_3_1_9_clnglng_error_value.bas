' INV: vbal.6.1.2.3.1.9, func.clnglng
' SOURCE: spec ms-vbal 6.1.2.3.1.9 CLngLng of an Error value converts its error code
' EXPECT: err9=9
Public Sub Main()
    Debug.Print "err9=" & CStr(CLngLng(CVErr(9)))
End Sub

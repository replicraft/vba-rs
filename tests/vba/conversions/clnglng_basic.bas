' INV: func.clnglng
' SOURCE: spec ms-vbal 6.1.2.3.1.9 CLngLng
' EXPECT: 1099511627776
' EXPECT: 8
' EXPECT: LongLong
Public Sub Main()
    Debug.Print CStr(CLngLng(2 ^ 40))
    Debug.Print CStr(CLngLng(7.5))
    Debug.Print CStr(TypeName(CLngLng(1)))
End Sub

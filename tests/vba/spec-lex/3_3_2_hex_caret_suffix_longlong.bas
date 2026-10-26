' INV: vbal.3.3.2, lit.hex
' SOURCE: spec ms-vbal 3.3.2 number tokens, hex literal with ^ suffix is LongLong with value n
' EXPECT: 32767
' EXPECT: 65535
' EXPECT: 2147483647
' EXPECT: 2147483648
' EXPECT: LongLong
Public Sub Main()
    Dim x As LongLong
    x = &H7FFF^
    Debug.Print CStr(x)
    x = &HFFFF^
    Debug.Print CStr(x)
    x = &H7FFFFFFF^
    Debug.Print CStr(x)
    x = &H80000000^
    Debug.Print CStr(x)
    Debug.Print TypeName(x)
End Sub

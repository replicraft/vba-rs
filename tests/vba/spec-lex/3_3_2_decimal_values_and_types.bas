' INV: vbal.3.3.2, lit.integer
' SOURCE: spec ms-vbal 3.3.2 number tokens, decimal literal types by range and suffix
' EXPECT: Integer
' EXPECT: Integer
' EXPECT: Long
' EXPECT: Long
' EXPECT: Double
' EXPECT: 32768
' EXPECT: 32768
' EXPECT: 5000000000
' EXPECT: 3000000001
' EXPECT: 7
Public Sub Main()
    Debug.Print TypeName(32767)
    Debug.Print TypeName(32767%)
    Debug.Print TypeName(32768)
    Debug.Print TypeName(2147483647)
    Debug.Print TypeName(3000000000)
    Debug.Print CStr(32768&)
    Debug.Print CStr(32768^)
    Debug.Print CStr(5000000000^)
    Debug.Print CStr(3000000000 + 1)
    Debug.Print CStr(007)
End Sub

' INV: vbal.3.3.2, lit.hex
' SOURCE: spec ms-vbal 3.3.2 number tokens, hex literal without suffix: value and type by range
' EXPECT: 32767 Integer
' EXPECT: -32768 Integer
' EXPECT: -1 Integer
' EXPECT: 65536 Long
' EXPECT: 2147483647 Long
' EXPECT: -2147483648 Long
' EXPECT: -1 Long
Public Sub Main()
    Debug.Print CStr(&H7FFF) & " " & TypeName(&H7FFF)
    Debug.Print CStr(&H8000) & " " & TypeName(&H8000)
    Debug.Print CStr(&HFFFF) & " " & TypeName(&HFFFF)
    Debug.Print CStr(&H10000) & " " & TypeName(&H10000)
    Debug.Print CStr(&H7FFFFFFF) & " " & TypeName(&H7FFFFFFF)
    Debug.Print CStr(&H80000000) & " " & TypeName(&H80000000)
    Debug.Print CStr(&HFFFFFFFF) & " " & TypeName(&HFFFFFFFF)
End Sub

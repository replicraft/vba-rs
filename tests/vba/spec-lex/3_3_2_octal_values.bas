' INV: vbal.3.3.2, lit.octal
' SOURCE: spec ms-vbal 3.3.2 number tokens, octal literal value and type by range and suffix
' EXPECT: 32767 Integer
' EXPECT: -32768 Integer
' EXPECT: -1 Integer
' EXPECT: 65536 Long
' EXPECT: -2147483648 Long
' EXPECT: -1 Long
' EXPECT: 65535
' EXPECT: -1
' EXPECT: -1
Public Sub Main()
    Debug.Print CStr(&O77777) & " " & TypeName(&O77777)
    Debug.Print CStr(&O100000) & " " & TypeName(&O100000)
    Debug.Print CStr(&O177777) & " " & TypeName(&O177777)
    Debug.Print CStr(&O200000) & " " & TypeName(&O200000)
    Debug.Print CStr(&O20000000000) & " " & TypeName(&O20000000000)
    Debug.Print CStr(&O37777777777) & " " & TypeName(&O37777777777)
    Debug.Print CStr(&O177777&)
    Debug.Print CStr(&O177777%)
    Debug.Print CStr(&O37777777777&)
End Sub

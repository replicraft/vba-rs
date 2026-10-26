' INV: vbal.5.6.9.5, op.eq, op.lt
' SOURCE: spec ms-vbal 5.6.9.5 relational LongLong effective type compares exactly, with String and integral
' EXPECT: False
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim a As LongLong, b As LongLong, n As Long
    a = 9223372036854775806^
    b = 9223372036854775807^
    Debug.Print CStr(a = b)
    Debug.Print CStr(a < b)
    Debug.Print CStr(a = "9223372036854775806")
    Debug.Print CStr(a <> "9223372036854775807")
    n = 5
    Debug.Print CStr(n < a)
End Sub

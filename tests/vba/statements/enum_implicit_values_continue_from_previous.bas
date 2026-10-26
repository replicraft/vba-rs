' INV: stmt.enum
' SOURCE: spec ms-vbal 5.2.3.4 an Enum member without a value is the previous member plus one, the first being 0
' EXPECT: 0 5 6 -3 -2 -1
' EXPECT: True
Private Enum E
    A
    B = 5
    C
    D = -3
    F
    G
End Enum
Public Sub Main()
    Debug.Print CStr(A) & " " & CStr(B) & " " & CStr(C) & " " & CStr(D) & " " & CStr(F) & " " & CStr(G)
    Dim x As E
    x = C
    Debug.Print CStr(x = 6)
End Sub

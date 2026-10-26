' INV: vbal.2.2
' SOURCE: spec ms-vbal 2.2 entities and declared types, Enum members are Long values and an Enum-declared variable may hold any Long value
' EXPECT: Long
' EXPECT: Long
' EXPECT: 0
' EXPECT: 99
Private Enum Color
    Red = 1
    Green = 2
End Enum
Public Sub Main()
    Dim c As Color
    Debug.Print TypeName(Red)
    Debug.Print TypeName(c)
    Debug.Print CStr(c)
    c = 99
    Debug.Print CStr(c)
End Sub

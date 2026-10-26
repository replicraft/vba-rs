' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 declared type of an enum member is Long
' EXPECT: Long
' EXPECT: Long
' EXPECT: 3
Private Enum Sizes
    Small = 1
    Big = 100000
End Enum
Public Sub Main()
    Debug.Print TypeName(Small)
    Debug.Print TypeName(Big)
    Dim s As Sizes
    s = Small + 2
    Debug.Print CStr(s)
End Sub

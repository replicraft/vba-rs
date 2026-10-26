' INV: vbal.5.6.12
' SOURCE: spec ms-vbal 5.6.12 member access on an Enum type yields the enum member as a value
' EXPECT: 20
' EXPECT: 30
Private Enum Level
    Low = 10
    Mid = 20
    High = 30
End Enum
Public Sub Main()
    Debug.Print CStr(Level.Mid)
    Debug.Print CStr(Level.High)
End Sub

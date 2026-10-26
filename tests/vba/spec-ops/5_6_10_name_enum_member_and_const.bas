' INV: vbal.5.6.10
' SOURCE: spec ms-vbal 5.6.10 Enum members and constants resolve as value expressions
' EXPECT: 2
' EXPECT: 10
' EXPECT: 3
Private Enum Color
    Red = 1
    Green
    Blue
End Enum
Private Const Ten As Long = 10
Public Sub Main()
    Debug.Print CStr(Green)
    Debug.Print CStr(Ten)
    Debug.Print CStr(Color.Blue)
End Sub

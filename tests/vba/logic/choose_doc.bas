' INV: func.choose
' SOURCE: spec ms-vbal 6.1.2.8.1.2 Choose
' EXPECT: three
' EXPECT: Speedy
' EXPECT: 20
Public Sub Main()
    Debug.Print CStr(Choose(3, "one", "two", "three"))
    Debug.Print CStr(Choose(1, "Speedy", "United", "Federal"))
    Debug.Print CStr(Choose(2.4, 10, 20, 30))
End Sub

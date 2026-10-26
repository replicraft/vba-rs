' INV: func.right
' SOURCE: spec ms-vbal 6.1.2.11.1.30 Right length cases
' EXPECT: d
' EXPECT:  World
' EXPECT: Hello World
Public Sub Main()
    Debug.Print CStr(Right("Hello World", 1))
    Debug.Print CStr(Right("Hello World", 6))
    Debug.Print CStr(Right("Hello World", 20))
End Sub

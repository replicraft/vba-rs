' INV: func.left
' SOURCE: spec ms-vbal 6.1.2.11.1.19 Left length cases
' EXPECT: H
' EXPECT: Hello W
' EXPECT: Hello World
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(Left("Hello World", 1))
    Debug.Print CStr(Left("Hello World", 7))
    Debug.Print CStr(Left("Hello World", 20))
    Debug.Print CStr(Left("Hello World", 0))
End Sub

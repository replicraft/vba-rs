' INV: vbal.5.2.1.1
' SOURCE: spec ms-vbal 5.2.1.1 Option Compare Text relational operators
' EXPECT: eq=True
' EXPECT: lt=True
' EXPECT: ne=False
' EXPECT: gt=False
Option Compare Text
Public Sub Main()
    Debug.Print "eq=" & CStr("abc" = "ABC")
    Debug.Print "lt=" & CStr("a" < "B")
    Debug.Print "ne=" & CStr("abc" <> "ABC")
    Debug.Print "gt=" & CStr("a" > "B")
End Sub

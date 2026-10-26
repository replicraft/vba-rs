' INV: obj.debug
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/debug-object
' EXPECT: a             b
' EXPECT: ab
' EXPECT:  1  2 
' EXPECT: 
' EXPECT: x
Public Sub Main()
    Debug.Print "a", "b"
    Debug.Print "a"; "b"
    Debug.Print 1; 2
    Debug.Print
    Debug.Print "x"
End Sub

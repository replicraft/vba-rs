' INV: func.split
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/split-function
' EXPECT: -1 
' EXPECT:  0 abc
' EXPECT:  2 
' EXPECT:  1 b,c,d
Public Sub Main()
    Dim a() As String
    a = Split("")
    Debug.Print UBound(a)
    a = Split("abc", "")
    Debug.Print UBound(a); a(0)
    a = Split("a b c")
    Debug.Print UBound(a)
    a = Split("a,b,c,d", ",", 2)
    Debug.Print UBound(a); a(1)
End Sub

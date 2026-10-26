' INV: func.split, func.lbound
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/split-function
' EXPECT:  0  2 
Public Sub Main()
    Dim a() As String
    a = Split("a b c")
    Debug.Print LBound(a); UBound(a)
End Sub

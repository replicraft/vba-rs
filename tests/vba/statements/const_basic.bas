' INV: stmt.const
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/const-statement
' EXPECT:  3  6 hi
Public Sub Main()
    Const A = 3
    Const B As Long = A * 2
    Const S = "hi"
    Debug.Print A; B; S
End Sub

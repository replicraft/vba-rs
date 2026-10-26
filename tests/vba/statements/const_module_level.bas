' INV: stmt.const, decl.scope
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/const-statement
' EXPECT:  100 x
Private Const LIMIT As Long = 100
Public Const NAME_ = "x"
Public Sub Main()
    Debug.Print LIMIT; NAME_
End Sub

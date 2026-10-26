' INV: vbal.3.2.2
' SOURCE: spec ms-vbal 3.2.2 logical line grammar, a tab is white space (WSC) separating tokens
' EXPECT: 12
Public Sub Main()
    Dim	x	As	Long
    x	=	5	+	7
    Debug.Print	CStr(x)
End Sub

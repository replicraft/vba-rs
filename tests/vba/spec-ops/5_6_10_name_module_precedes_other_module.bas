' INV: vbal.5.6.10, decl.scope
' SOURCE: spec ms-vbal 5.6.10 simple name tiers: enclosing module precedes other procedural modules
' EXPECT: mine
' EXPECT: other
'#MODULE Other standard
Public Function Name1() As String
    Name1 = "other"
End Function
'#MODULE Module1 standard
Private Function Name1() As String
    Name1 = "mine"
End Function
Public Sub Main()
    Debug.Print Name1()
    Debug.Print Other.Name1()
End Sub

' INV: vbal.6.1.2.8.1.16, func.switch
' SOURCE: spec ms-vbal 6.1.2.8.1.16 Switch evaluates all expressions left to right even after a True one
' EXPECT: e1
' EXPECT: v1
' EXPECT: e2
' EXPECT: v2
' EXPECT: r=one
Private Function P(ByVal tag As String, ByVal x As Variant) As Variant
    Debug.Print tag
    P = x
End Function
Public Sub Main()
    Debug.Print "r=" & Switch(P("e1", True), P("v1", "one"), P("e2", True), P("v2", "two"))
End Sub

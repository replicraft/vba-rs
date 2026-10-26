' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, an excluded block may hold whole declarations; alternative definitions of one name
' EXPECT: second
#If False Then
Private Function Which() As String
    Which = "first"
End Function
#Else
Private Function Which() As String
    Which = "second"
End Function
#End If
Public Sub Main()
    Debug.Print Which()
End Sub

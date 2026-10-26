' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, lines of excluded blocks are not part of the program
' EXPECT: ok
Public Sub Main()
#If False Then
    This is not valid VBA at all = = (((
    Call NoSuchProcedure(1, 2
    Dim As As As
#ElseIf False Then
    also not valid + * /
#Else
    Debug.Print "ok"
#End If
#If True Then
#Else
    Next Without For
    End Function
#End If
End Sub

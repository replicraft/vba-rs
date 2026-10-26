' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, a true #If block excludes later true #ElseIf and #Else blocks
' EXPECT: if
Public Sub Main()
#If True Then
    Debug.Print "if"
#ElseIf True Then
    Debug.Print "elseif"
#Else
    Debug.Print "else"
#End If
End Sub

' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, with no true expression and no #Else there is no included block; #EndIf spelling
' EXPECT: after
Public Sub Main()
#If False Then
    Debug.Print "if"
#ElseIf 0 Then
    Debug.Print "elseif"
#EndIf
    Debug.Print "after"
End Sub

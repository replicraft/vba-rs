' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, numeric expression values are Let-coerced to Boolean (nonzero is true)
' EXPECT: five true
' EXPECT: minus one true
' EXPECT: zero false
Public Sub Main()
#If 5 Then
    Debug.Print "five true"
#Else
    Debug.Print "five false"
#End If
#If -1 Then
    Debug.Print "minus one true"
#Else
    Debug.Print "minus one false"
#End If
#If 0 Then
    Debug.Print "zero true"
#Else
    Debug.Print "zero false"
#End If
End Sub

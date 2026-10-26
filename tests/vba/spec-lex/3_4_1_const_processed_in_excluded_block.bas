' INV: vbal.3.4.1, cc.const
' SOURCE: spec ms-vbal 3.4.1 conditional compilation const directive, all #Const directives are processed including those in excluded blocks
' EXPECT: b is 7
#Const A = 1
#If A = 2 Then
#Const B = 7
#End If
Public Sub Main()
#If B = 7 Then
    Debug.Print "b is 7"
#Else
    Debug.Print "b is not 7"
#End If
End Sub

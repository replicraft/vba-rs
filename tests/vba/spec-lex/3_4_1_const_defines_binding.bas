' INV: vbal.3.4.1, cc.const
' SOURCE: spec ms-vbal 3.4.1 conditional compilation const directive, a #Const binds the value of its expression for later directives
' EXPECT: 7
' EXPECT: 14
#Const A = 7
#Const B = A * 2
Public Sub Main()
    Dim x As Long
#If A = 7 Then
    x = 7
#End If
    Debug.Print CStr(x)
#If B = 14 Then
    x = 14
#End If
    Debug.Print CStr(x)
End Sub

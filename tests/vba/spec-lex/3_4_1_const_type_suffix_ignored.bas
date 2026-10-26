' INV: vbal.3.4.1, cc.const
' SOURCE: spec ms-vbal 3.4.1 conditional compilation const directive, a type suffix on the constant name is ignored
' EXPECT: yes
#Const N% = 3
#Const T$ = "text"
Public Sub Main()
#If N = 3 And T = "text" Then
    Debug.Print "yes"
#Else
    Debug.Print "no"
#End If
End Sub

' INV: vbal.3.4.2, cc.if
' SOURCE: spec ms-vbal 3.4.2 conditional compilation if directives, cc-expression is a constant expression evaluated at preprocessing time
' EXPECT: 1 2 3 4 5
#Const N = 7
#Const S = "abc"
Public Sub Main()
    Dim r As String
#If 2 + 3 = 5 And Not False Then
    r = r & "1 "
#End If
#If (N Mod 2) = 1 Or False Then
    r = r & "2 "
#End If
#If S = "abc" Then
    r = r & "3 "
#End If
#If N > 6 And N < 8 Then
    r = r & "4 "
#End If
#If N \ 2 = 3 Then
    r = r & "5"
#End If
    Debug.Print r
End Sub

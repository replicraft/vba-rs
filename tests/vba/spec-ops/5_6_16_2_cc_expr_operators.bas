' INV: vbal.5.6.16.2, cc.const
' SOURCE: spec ms-vbal 5.6.16.2 conditional compilation expressions with operators and cc constants
' XFAIL: wrong pending
' EXPECT: a
' EXPECT: b
' EXPECT: c
' EXPECT: d
' EXPECT: e
#Const K1 = 2 + 3 * 4
#Const K2 = (K1 - 4) \ 2
#Const K3 = "ab" & "cd"
#Const K4 = Not (K1 = 14) Or K2 = 5
#Const K5 = Len("abc") + Abs(-2)
Public Sub Main()
#If K1 = 14 Then
    Debug.Print "a"
#Else
    Debug.Print "A?"
#End If
#If K2 = 5 Then
    Debug.Print "b"
#Else
    Debug.Print "B?"
#End If
#If K3 = "abcd" Then
    Debug.Print "c"
#Else
    Debug.Print "C?"
#End If
#If K4 Then
    Debug.Print "d"
#Else
    Debug.Print "D?"
#End If
#If K5 = 5 And K3 Like "a*" Then
    Debug.Print "e"
#Else
    Debug.Print "E?"
#End If
End Sub

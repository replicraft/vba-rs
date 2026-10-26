' INV: vbal.3.4.1, cc.const
' SOURCE: spec ms-vbal 3.4.1 conditional compilation const directive, #Const is a cc-block element and may occur between the logical lines of a procedure
' EXPECT: v=2
Public Sub Main()
#Const V = 2
#If V = 2 Then
    Debug.Print "v=2"
#Else
    Debug.Print "v=3"
#End If
End Sub

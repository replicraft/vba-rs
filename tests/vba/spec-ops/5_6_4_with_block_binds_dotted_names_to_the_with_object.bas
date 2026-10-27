' INV: vbal.5.6.4
' SOURCE: derived 5.6.4 inside With, a name that begins with a period is bound against the With object
' METHOD: by hand: 2 then 5
' EXPECT:  2 
' EXPECT:  5 
'#MODULE Box class
Public V As Long
Public Function Plus(ByVal n As Long) As Long
    Plus = V + n
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box
    With b
        .V = 2
        Debug.Print .V
        Debug.Print .Plus(3)
    End With
End Sub

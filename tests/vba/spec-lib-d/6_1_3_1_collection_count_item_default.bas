' INV: vbal.6.1.3.1.1.1, vbal.6.1.3.1.1.2, type.collection
' SOURCE: spec ms-vbal 6.1.3.1.1 Collection Count (Long), Item by position and key, Item is the default member
' EXPECT: n=3
' EXPECT: t=Long
' EXPECT: i1=x
' EXPECT: i3=z
' EXPECT: k=y
' EXPECT: d=x
' EXPECT: d2=z
Public Sub Main()
    Dim c As New Collection
    c.Add "x"
    c.Add "y", "ky"
    c.Add "z"
    Debug.Print "n=" & CStr(c.Count)
    Debug.Print "t=" & TypeName(c.Count)
    Debug.Print "i1=" & c.Item(1)
    Debug.Print "i3=" & c.Item(3)
    Debug.Print "k=" & c.Item("ky")
    Debug.Print "d=" & c(1)
    Debug.Print "d2=" & c(3)
End Sub

' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 zero step is treated like a positive one when testing the end
' EXPECT: ran=0
Public Sub Main()
    Dim i As Long, c As Long
    For i = 5 To 1 Step 0
        c = c + 1
    Next i
    Debug.Print "ran=" & CStr(c)
End Sub

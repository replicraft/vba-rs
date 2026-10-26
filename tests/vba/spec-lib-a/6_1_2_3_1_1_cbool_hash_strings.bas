' INV: vbal.6.1.2.3.1.1, vbal.5.5.1.2.4, func.cbool
' SOURCE: spec ms-vbal 6.1.2.3.1.1 CBool of "#TRUE#" / "#FALSE#" strings (Let-coercion String to Boolean)
' EXPECT: hashTrue=True
' EXPECT: hashFalse=False
Public Sub Main()
    Debug.Print "hashTrue=" & CStr(CBool("#TRUE#"))
    Debug.Print "hashFalse=" & CStr(CBool("#FALSE#"))
End Sub
